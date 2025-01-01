use std::collections::HashMap;

use crate::client::Client;
use rustafarian_shared::assembler::{assembler::Assembler, disassembler::Disassembler};
use rustafarian_shared::messages::browser_messages::{
    BrowserRequest, BrowserRequestWrapper, BrowserResponse, BrowserResponseWrapper,
};
use rustafarian_shared::messages::commander_messages::{
    SimControllerCommand, SimControllerMessage, SimControllerResponseWrapper,
};
use rustafarian_shared::messages::general_messages::{
    DroneSend, ServerType, ServerTypeRequest, ServerTypeResponse,
};
use rustafarian_shared::topology::Topology;

use crossbeam_channel::{Receiver, Sender};
use wg_2024::{network::NodeId, packet::Packet};

pub struct BrowserClient {
    // Used for general client
    client_id: u8,
    senders: HashMap<u8, Sender<Packet>>,
    receiver: Receiver<Packet>,
    topology: Topology,
    sim_controller_receiver: Receiver<SimControllerCommand>,
    sim_controller_sender: Sender<SimControllerResponseWrapper>,
    sent_packets: HashMap<u64, Vec<Packet>>,
    acked_packets: HashMap<u64, Vec<bool>>,
    assembler: Assembler,
    disassembler: Disassembler,
    running: bool,
    packets_to_send: HashMap<u64, Packet>,

    // Specific to browser client
    available_text_files: HashMap<NodeId, Vec<u8>>, // The text files available from Text Content Servers
    available_media_files: HashMap<NodeId, Vec<u8>>, // The media files available from Media Content Servers
    obtained_text_files: HashMap<(NodeId, u8), Vec<u8>>, // The text files obtained from Text Content Servers. The key is a tuple of (server_id, file_id)
    obtained_media_files: HashMap<(NodeId, u8), Vec<u8>>, // The media files obtained from Media Content Servers. The key is a tuple of (server_id, file_id)
    available_servers: HashMap<NodeId, ServerType>, // The servers available to the browser client
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
            assembler: Assembler::new(),
            disassembler: Disassembler::new(),
            running: false,
            packets_to_send: HashMap::new(),

            available_text_files: HashMap::new(),
            available_media_files: HashMap::new(),
            obtained_text_files: HashMap::new(),
            obtained_media_files: HashMap::new(),
            available_servers: HashMap::new(),
        }
    }

    /// Requests a text file from a server
    pub fn request_text_file(&mut self, file_id: u8, server_id: NodeId) {
        let request = BrowserRequestWrapper::Chat(BrowserRequest::TextFileRequest(file_id));
        let request_json = request.stringify();
        self.send_message(server_id, request_json);
    }

    /// Requests a media file from a server
    pub fn request_media_file(&mut self, file_id: u8, server_id: NodeId) {
        let request = BrowserRequestWrapper::Chat(BrowserRequest::MediaFileRequest(file_id));
        let request_json = request.stringify();
        self.send_message(server_id, request_json);
    }

    /// Requests a list of files from a server
    pub fn request_file_list(&mut self, server_id: NodeId) {
        let request = BrowserRequestWrapper::Chat(BrowserRequest::FileList);
        let request_json = request.stringify();
        self.send_message(server_id, request_json);
    }

    /// Handle a response from a server
    fn handle_browser_response(&mut self, response: BrowserResponse, server_id: NodeId) {
        match response {
            // If the response is a list of files, add it to the available files
            BrowserResponse::FileList(files) => {
                match self.available_servers.get(&server_id) {
                    Some(server_type) => {
                        if matches!(server_type, ServerType::Text) {
                            self.available_text_files.insert(server_id, files.clone());
                        } else if matches!(server_type, ServerType::Media) {
                            self.available_media_files.insert(server_id, files.clone());
                        }
                    }
                    None => {
                        eprintln!("Server type not found for server_id: {}", server_id);
                    }
                }
                let files_str = String::from_utf8(files.clone()).unwrap();
                println!("Files: {}", files_str);

                // Send the list of files to the sim controller
                let _res = self
                    .sim_controller_sender
                    .send(SimControllerResponseWrapper::Message(
                        SimControllerMessage::FileListResponse(files),
                    ));
            }
            // If the response is a text file, add it to the obtained text files
            BrowserResponse::TextFile(file_id, text) => {
                self.obtained_text_files
                    .insert((server_id, file_id), text.as_bytes().to_vec());

                println!("Text: {}", text);
                // Send the text file to the sim controller
                let _res = self
                    .sim_controller_sender
                    .send(SimControllerResponseWrapper::Message(
                        SimControllerMessage::TextFileResponse(file_id, text),
                    ));
            }
            // If the response is a media file, add it to the obtained media files
            BrowserResponse::MediaFile(file_id, media) => {
                self.obtained_media_files
                    .insert((server_id, file_id), media.clone());
                println!("Media: {:?}", media);

                // Send the media file to the sim controller
                let _res = self
                    .sim_controller_sender
                    .send(SimControllerResponseWrapper::Message(
                        SimControllerMessage::MediaFileResponse(file_id, media),
                    ));
            }
        };
    }

    pub fn get_available_text_files(&self) -> &HashMap<NodeId, Vec<u8>> {
        &self.available_text_files
    }

    pub fn get_available_media_files(&self) -> &HashMap<NodeId, Vec<u8>> {
        &self.available_media_files
    }

    pub fn get_obtained_text_files(&self) -> &HashMap<(NodeId, u8), Vec<u8>> {
        &self.obtained_text_files
    }

    pub fn get_obtained_media_files(&self) -> &HashMap<(NodeId, u8), Vec<u8>> {
        &self.obtained_media_files
    }

    pub fn get_available_servers(&self) -> &HashMap<NodeId, ServerType> {
        &self.available_servers
    }
}

impl Client for BrowserClient {
    type RequestType = BrowserRequestWrapper;
    type ResponseType = BrowserResponseWrapper;

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

    /// Handle a response from a server
    fn handle_response(&mut self, response: Self::ResponseType, server_id: NodeId) {
        match response {
            // If the response is a normal BrowserResponse, handle it
            BrowserResponseWrapper::Chat(response) => {
                self.handle_browser_response(response, server_id)
            }
            // If the response is a ServerTypeResponse, handle it
            BrowserResponseWrapper::ServerType(server_response) => {
                let ServerTypeResponse::ServerType(server_response) = server_response;
                println!("Server response: {:?}", server_response);
                // If it's not a chat server, add it to the available servers (as a key of available_files)
                match server_response {
                    ServerType::Text => {
                        self.available_servers.insert(server_id, ServerType::Text);
                        self.available_text_files.insert(server_id, vec![]);
                    }
                    ServerType::Media => {
                        self.available_servers.insert(server_id, ServerType::Media);
                        self.available_media_files.insert(server_id, vec![]);
                    }
                    _ => {}
                }
                
                // Send the server type response to the sim controller
                let response = SimControllerMessage::ServerTypeResponse(server_id, server_response);
                self.sim_controller_sender
                    .send(SimControllerResponseWrapper::Message(response))
                    .unwrap();
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

    fn handle_controller_commands(&mut self, command: SimControllerCommand) {
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
            SimControllerCommand::AddSender(sender_id, sender_channel) => {
                self.senders.insert(sender_id, sender_channel);
                self.topology.add_node(sender_id);
                self.topology.add_edge(self.client_id, sender_id);
            }
            SimControllerCommand::RemoveSender(sender_id) => {
                self.senders.remove(&sender_id);
                self.topology.remove_node(sender_id);
            }
            _ => {
                eprintln!(
                    "Requesting Chat Client commands on Browser Client?! ({:?})",
                    command
                );
            }
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

    fn packets_to_send(&mut self) -> &mut HashMap<u64, Packet> {
        &mut self.packets_to_send
    }
}
