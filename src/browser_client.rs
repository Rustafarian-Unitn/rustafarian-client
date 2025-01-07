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
    packets_to_send: HashMap<u8, Packet>,
    sent_flood_ids: Vec<u64>,
    last_flood_timestamp: u128,

    // Specific to browser client
    /// The text files available from Text Content Servers
    available_text_files: HashMap<NodeId, Vec<u8>>,
    /// The media files available from Media Content Servers
    available_media_files: HashMap<NodeId, Vec<u8>>,
    /// The text files obtained from Text Content Servers. The key is a tuple of (server_id, file_id)
    obtained_text_files: HashMap<(NodeId, u8), Vec<u8>>,
    /// The media files obtained from Media Content Servers. The key is a tuple of (server_id, file_id)
    obtained_media_files: HashMap<(NodeId, u8), Vec<u8>>,
    /// The servers available to the browser client
    available_servers: HashMap<NodeId, ServerType>,
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
            sent_flood_ids: Vec::new(),
            last_flood_timestamp: 0,

            available_text_files: HashMap::new(),
            available_media_files: HashMap::new(),
            obtained_text_files: HashMap::new(),
            obtained_media_files: HashMap::new(),
            available_servers: HashMap::new(),
        }
    }

    /// Requests a text file from a server
    pub fn request_text_file(&mut self, file_id: u8, server_id: NodeId) {
        println!(
            "Client {} requesting text file {} from server {}",
            self.client_id, file_id, server_id
        );
        let request = BrowserRequestWrapper::Chat(BrowserRequest::TextFileRequest(file_id));
        let request_json = request.stringify();
        self.send_message(server_id, request_json);
    }

    /// Requests a media file from a server
    pub fn request_media_file(&mut self, file_id: u8, server_id: NodeId) {
        println!(
            "Client {} requesting media file {} from server {}",
            self.client_id, file_id, server_id
        );
        let request = BrowserRequestWrapper::Chat(BrowserRequest::MediaFileRequest(file_id));
        let request_json = request.stringify();
        self.send_message(server_id, request_json);
    }

    /// Requests a list of files from a server
    pub fn request_file_list(&mut self, server_id: NodeId) {
        println!(
            "Client {} requesting file list from server {}",
            self.client_id, server_id
        );
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
                println!(
                    "Client {} received file list from server {}: {:?}",
                    self.client_id, server_id, files
                );

                // Send the list of files to the sim controller
                match self
                    .sim_controller_sender
                    .send(SimControllerResponseWrapper::Message(
                        SimControllerMessage::FileListResponse(server_id, files),
                    )) {
                    Ok(_) => {
                        println!(
                            "Client {}: File list response sent to sim controller",
                            self.client_id
                        );
                    }
                    Err(e) => {
                        eprintln!(
                            "Client {}: Error sending file list response to sim controller: {}",
                            self.client_id, e
                        );
                    }
                };
            }
            // If the response is a text file, add it to the obtained text files
            BrowserResponse::TextFile(file_id, text) => {
                self.obtained_text_files
                    .insert((server_id, file_id), text.as_bytes().to_vec());

                println!(
                    "Client {} received text file from {}: {}",
                    self.client_id, server_id, text
                );
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
                println!(
                    "Client {} received media file from server {}: {:?}",
                    self.client_id, server_id, media
                );

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
                self.topology()
                    .set_node_type(server_id, format!("{:?}", server_response));
                println!(
                    "Client {} received server type: {:?} from {:?}",
                    self.client_id, server_response, server_id
                );
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
                    _ => {
                        println!(
                            "Client {}: Server type 'Chat' not added to available servers",
                            self.client_id
                        );
                    }
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
            // If the command is a request for the file list, send the request
            SimControllerCommand::RequestFileList(server_id) => {
                println!("COMMAND: Requesting file list from server {}", server_id);
                self.request_file_list(server_id);
            }
            // If the command is a request for a text file, send the request
            SimControllerCommand::RequestTextFile(file_id, server_id) => {
                println!(
                    "COMMAND: Requesting text file {} from server {}",
                    file_id, server_id
                );
                self.request_text_file(file_id, server_id);
            }
            // If the command is a request for a media file, send the request
            SimControllerCommand::RequestMediaFile(file_id, server_id) => {
                println!(
                    "COMMAND: Requesting media file {} from server {}",
                    file_id, server_id
                );
                self.request_media_file(file_id, server_id);
            }
            // If the command is a flood request, send the request
            SimControllerCommand::FloodRequest => {
                println!("COMMAND: Sending flood request");
                self.send_flood_request();
            }
            // If the command is a topology request, send the topology
            SimControllerCommand::Topology => {
                let topology = self.topology.clone();
                println!("COMMAND: Sending topology {:?}", topology);
                let response = SimControllerMessage::TopologyResponse(topology);
                self.sim_controller_sender
                    .send(SimControllerResponseWrapper::Message(response))
                    .unwrap();
            }
            // If the command is to add a neighbor, add the neighbor to the map and the topology
            SimControllerCommand::AddSender(sender_id, sender_channel) => {
                println!("COMMAND: Adding sender {}", sender_id);
                self.senders.insert(sender_id, sender_channel);
                self.topology.add_node(sender_id);
                self.topology.add_edge(self.client_id, sender_id);
            }
            // If the command is to remove a neighbor, remove the neighbor from the map and the topology
            SimControllerCommand::RemoveSender(sender_id) => {
                println!("COMMAND: Removing sender {}", sender_id);
                self.senders.remove(&sender_id);
                self.topology.remove_node(sender_id);
            }
            // If the command wants the servers known by the client, send the known servers
            SimControllerCommand::KnownServers => {
                println!(
                    "COMMAND: Sending known servers ({:?})",
                    self.available_servers
                );
                let known_servers = self.available_servers.clone();
                let response = SimControllerMessage::KnownServers(known_servers);
                self.sim_controller_sender
                    .send(SimControllerResponseWrapper::Message(response))
                    .unwrap();
            }
            SimControllerCommand::RequestServerType(server_id) => {
                println!("COMMAND: Requesting server type from server {}", server_id);
                self.send_server_type_request(server_id);
            }
            // Commands related to the Chat Client
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
        println!(
            "Client {} sending server type request to server {}",
            self.client_id, server_id
        );
        let request = ServerTypeRequest::ServerType;
        let request_wrapped = BrowserRequestWrapper::ServerType(request);
        let request_json = request_wrapped.stringify();
        self.send_message(server_id, request_json);
    }

    fn running(&mut self) -> &mut bool {
        &mut self.running
    }

    fn packets_to_send(&mut self) -> &mut HashMap<u8, Packet> {
        &mut self.packets_to_send
    }

    fn sent_flood_ids(&mut self) -> &mut Vec<u64> {
        &mut self.sent_flood_ids
    }

    fn last_flood_timestamp(&mut self) -> &mut u128 {
        &mut self.last_flood_timestamp
    }
}
