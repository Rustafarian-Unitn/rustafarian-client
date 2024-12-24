use std::collections::HashMap;

use crate::client::Client;
use rustafarian_shared::assembler::{assembler::Assembler, disassembler::Disassembler};
use rustafarian_shared::messages::browser_messages::{
    BrowserRequest, BrowserRequestWrapper, BrowserResponse, BrowserResponseWrapper
};
use rustafarian_shared::messages::commander_messages::{
    SimControllerCommand, SimControllerMessage, SimControllerResponseWrapper,
};
use rustafarian_shared::messages::general_messages::{DroneSend, Message, ServerTypeRequest};
use rustafarian_shared::topology::Topology;

use crossbeam_channel::{Receiver, Sender};
use wg_2024::{network::NodeId, packet::Packet};

pub struct BrowserClient {
    client_id: u8,
    senders: HashMap<u8, Sender<Packet>>,
    receiver: Receiver<Packet>,
    topology: Topology,
    sim_controller_receiver: Receiver<SimControllerCommand>,
    sim_controller_sender: Sender<SimControllerResponseWrapper>,
    sent_packets: HashMap<u64, Vec<Packet>>,
    acked_packets: HashMap<u64, Vec<bool>>,
    available_files: HashMap<NodeId, Vec<u8>>, // Key: server_id, value: list of file ids
    assembler: Assembler,
    disassembler: Disassembler,
    running: bool,
}

impl BrowserClient {
    pub fn new(
        client_id: u8,
        senders: HashMap<u8, Sender<Packet>>,
        receiver: Receiver<Packet>,
        sim_controller_receiver: Receiver<SimControllerCommand>,
        sim_controller_sender: Sender<SimControllerResponseWrapper>,
    ) -> Self {
        BrowserClient {
            client_id,
            senders,
            receiver,
            topology: Topology::new(),
            sim_controller_receiver,
            sim_controller_sender,
            sent_packets: HashMap::new(),
            acked_packets: HashMap::new(),
            available_files: HashMap::new(),
            assembler: Assembler::new(),
            disassembler: Disassembler::new(),
            running: false,
        }
    }

    pub fn request_text_file(&mut self, file_id: u8, server_id: NodeId) {
        let request = BrowserRequestWrapper::Chat(
            BrowserRequest::TextFileRequest(file_id)
        );
        let request_json = request.stringify();
        self.send_message(server_id, request_json);
    }

    pub fn request_media_file(&mut self, file_id: u8, server_id: NodeId) {
        let request = BrowserRequestWrapper::Chat(
            BrowserRequest::MediaFileRequest(file_id)
        );
        let request_json = request.stringify();
        self.send_message(server_id, request_json);
    }

    pub fn request_file_list(&mut self, server_id: NodeId) {
        let request = BrowserRequestWrapper::Chat(
            BrowserRequest::FileList
        );
        let request_json = request.stringify();
        self.send_message(server_id, request_json);
    }

    fn handle_browser_response(&mut self, response: BrowserResponse, server_id: NodeId) {
        match response {
            BrowserResponse::FileList(files) => {
                let files_str = String::from_utf8(files.clone()).unwrap();
                println!("Files: {}", files_str);

                let _res = self.sim_controller_sender
                    .send(SimControllerResponseWrapper::Message(
                        SimControllerMessage::FileListResponse(files)
                    ));
            }
            BrowserResponse::TextFile(file_id, text) => {
                println!("Text: {}", text);

                let _res = self.sim_controller_sender
                    .send(SimControllerResponseWrapper::Message(
                        SimControllerMessage::TextFileResponse(file_id, text)
                    ));
            }
            BrowserResponse::MediaFile(file_id, media) => {
                println!("Media: {:?}", media);

                let _res = self.sim_controller_sender
                    .send(SimControllerResponseWrapper::Message(
                        SimControllerMessage::MediaFileResponse(file_id, media)
                    ));
            }
        };
    }
}

impl Client for BrowserClient {
    type RequestType = BrowserRequestWrapper;
    type ResponseType = BrowserResponseWrapper;
    type SimControllerCommand = SimControllerCommand;

    fn client_id(&self) -> u8 {
        self.client_id
    }

    fn senders(&self) -> &HashMap<u8, Sender<Packet>> {
        &self.senders
    }

    fn receiver(&self) -> &Receiver<Packet> {
        &self.receiver
    }

    fn topology(&mut self) -> &mut Topology {
        &mut self.topology
    }

    fn handle_response(&mut self, response: Self::ResponseType, server_id: NodeId) {
        match response {
            BrowserResponseWrapper::Chat(response) => {
                self.handle_browser_response(response, server_id)
            }
            BrowserResponseWrapper::ServerType(server_response) => {
                println!("Server response: {:?}", server_response);
                self.available_files.insert(server_id, vec![]);
            }
        }
    }

    fn sim_controller_receiver(&self) -> &Receiver<SimControllerCommand> {
        &self.sim_controller_receiver
    }

    fn assembler(&mut self) -> &mut Assembler {
        &mut self.assembler
    }

    fn deassembler(&mut self) -> &mut Disassembler {
        &mut self.disassembler
    }

    fn sent_packets(&mut self) -> &mut HashMap<u64, Vec<Packet>> {
        &mut self.sent_packets
    }

    fn sim_controller_sender(&self) -> &Sender<SimControllerResponseWrapper> {
        &self.sim_controller_sender
    }

    fn handle_controller_commands(&mut self, command: Self::SimControllerCommand) {
        match command {
            SimControllerCommand::RequestFileList(server_id) => {
                self.request_file_list(server_id);
            }
            SimControllerCommand::RequestTextFile(file_id, server_id) => {
                self.request_text_file(file_id, server_id);
            }
            SimControllerCommand::RequestMediaFile(file_id, server_id) => {
                self.request_media_file(file_id, server_id);
            }
            SimControllerCommand::FloodRequest => {
                self.send_flood_request();
            }
            SimControllerCommand::Topology => {
                let topology = self.topology.clone();
                let response = SimControllerMessage::TopologyResponse(topology);
                self.sim_controller_sender
                    .send(SimControllerResponseWrapper::Message(response))
                    .unwrap();
            }
            _ => {}
        }
    }

    fn acked_packets(&mut self) -> &mut HashMap<u64, Vec<bool>> {
        &mut self.acked_packets
    }

    fn send_server_type_request(&mut self, server_id: NodeId) {
        let request = ServerTypeRequest::ServerType;
        let request_wrapped = BrowserRequestWrapper::ServerType(request);
        let request_json = request_wrapped.stringify();
        self.send_message(server_id, request_json);
    }

    fn running(&mut self) -> &mut bool {
        &mut self.running
    }
}
