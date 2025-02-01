use std::collections::{HashMap, HashSet};
use std::process;

use crate::client::Client;
use rustafarian_shared::assembler::{assembler::Assembler, disassembler::Disassembler};
use rustafarian_shared::logger::{LogLevel, Logger};
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
    logger: Logger,

    // Specific to browser client
    /// The text files available from Text Content Servers
    available_text_files: HashMap<NodeId, Vec<u8>>,
    /// The media files available from Media Content Servers
    available_media_files: HashMap<NodeId, Vec<u8>>,
    /// The text files obtained from Text Content Servers. The key is a tuple of (`server_id`, `file_id`)
    obtained_text_files: HashMap<(NodeId, u8), String>,
    /// The media files obtained from Media Content Servers. The key is a tuple of (`server_id`, `file_id`)
    obtained_media_files: HashMap<u8, Vec<u8>>,
    /// The servers available to the browser client
    available_servers: HashMap<NodeId, ServerType>,
    /// Files with references that are waiting for the referenced files to be obtained
    /// The key is the `file_id` of the file with references, and the value is a `HashSet` of the `file_ids` of the referenced files
    pending_referenced_files: HashMap<u8, HashSet<u8>>,
    /// This is the same as above, but the `HashSet` doesn't get updated every time a file is obtained
    /// It is needed to know which files to send with the text file to the sim controller
    references_files: HashMap<u8, HashSet<u8>>,
}

impl BrowserClient {
    #[must_use]
    pub fn new(
        client_id: u8,
        senders: HashMap<u8, Sender<Packet>>,
        receiver: Receiver<Packet>,
        sim_controller_receiver: Receiver<SimControllerCommand>,
        sim_controller_sender: Sender<SimControllerResponseWrapper>,
        debug: bool,
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
            logger: Logger::new("BrowserClient".to_string(), client_id, debug),

            available_text_files: HashMap::new(),
            available_media_files: HashMap::new(),
            obtained_text_files: HashMap::new(),
            obtained_media_files: HashMap::new(),
            available_servers: HashMap::new(),
            pending_referenced_files: HashMap::new(),
            references_files: HashMap::new(),
        }
    }

    /// Requests a text file from a server
    pub fn request_text_file(&mut self, file_id: u8, server_id: NodeId) {
        self.logger.log(
            &format!("Requesting text file {file_id} from server {server_id}"),
            LogLevel::DEBUG,
        );
        let request = BrowserRequestWrapper::Chat(BrowserRequest::TextFileRequest(file_id));
        let request_json = request.stringify();
        self.send_message(server_id, request_json);
    }

    /// Requests a media file from a server
    pub fn request_media_file(&mut self, file_id: u8, server_id: NodeId) {
        self.logger.log(
            &format!("Requesting media file {file_id} from server {server_id}"),
            LogLevel::DEBUG,
        );
        let request = BrowserRequestWrapper::Chat(BrowserRequest::MediaFileRequest(file_id));
        let request_json = request.stringify();
        self.send_message(server_id, request_json);
    }

    /// Requests a list of files from a server
    pub fn request_file_list(&mut self, server_id: NodeId) {
        self.logger.log(
            &format!("Requesting file list from server {server_id}"),
            LogLevel::DEBUG,
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
                        self.logger.log(
                            &format!("Server type not found for server_id: {server_id}"),
                            LogLevel::ERROR,
                        );
                    }
                }
                self.logger.log(
                    &format!("Received file list from server {server_id}: {files:?}"),
                    LogLevel::DEBUG,
                );

                // Send the list of files to the sim controller
                let _res = self
                    .sim_controller_sender
                    .send(SimControllerResponseWrapper::Message(
                        SimControllerMessage::FileListResponse(server_id, files),
                    ));
            }
            // If the response is a text file, add it to the obtained text files
            BrowserResponse::TextFile(file_id, text) => {
                self.obtained_text_files
                    .insert((server_id, file_id), text.clone());

                self.logger.log(
                    &format!(
                        "Received text file from {server_id}: {}...",
                        &text.chars().take(10).collect::<String>()
                    ),
                    LogLevel::DEBUG,
                );

                // Handle media files referenced inside the text file
                self.send_referenced_files_requests(&text, file_id);
            }
            // If the response is a media file, add it to the obtained media files
            BrowserResponse::MediaFile(file_id, media) => {
                self.obtained_media_files.insert(file_id, media.clone());
                self.logger.log(
                    &format!("Received media file from {server_id}"),
                    LogLevel::DEBUG,
                );

                // Check if the media file is referenced in a text file
                let is_reference = self.check_referenced_media_received(file_id);

                // If it's a reference, don't send it to the sim controller
                if is_reference {
                    return;
                }

                // Send the media file to the sim controller
                let _res = self
                    .sim_controller_sender
                    .send(SimControllerResponseWrapper::Message(
                        SimControllerMessage::MediaFileResponse(file_id, media),
                    ));
            }
        };
    }

    /// When a media file is obtained, check if it is referenced in a text file
    /// In that case, if all the references are obtained, send the text file to the sim controller with the attached media files
    fn check_referenced_media_received(&mut self, media_file_id: u8) -> bool {
        // Browse the pending referenced files and check if the obtained media file is referenced
        let mut is_reference = false;
        let mut completed_text_files = vec![];
        self.logger.log(
            &format!(
                "Checking if media file is a reference in text files: {:?}",
                self.pending_referenced_files
            ),
            LogLevel::DEBUG,
        );
        for (file_id, references) in &mut self.pending_referenced_files {
            if references.contains(&media_file_id) {
                self.logger.log(
                    &format!("Media file {media_file_id} is a reference in text file {file_id}"),
                    LogLevel::DEBUG,
                );
                is_reference = true;
                // Remove the reference from the pending_referenced_files map
                references.remove(&media_file_id);
                // If there are no more references, add the file_id to the completed_text_files
                if references.is_empty() {
                    completed_text_files.push(*file_id);
                }
            }
        }
        self.logger.log(
            &format!("Completed text files: {completed_text_files:?}"),
            LogLevel::DEBUG,
        );
        // Remove all the completed text files from the pending_referenced_files map
        // Then, send the completed file to the simulation controller
        for file_id in completed_text_files {
            let empty_string = String::new();
            let text = self
                .obtained_text_files
                .iter()
                .find(|k| k.0 .1 == file_id)
                .unwrap_or((&(0, 0), &empty_string));
            if text.0 == &(0, 0) {
                self.logger
                    .log(&format!("Text file {file_id} not found"), LogLevel::ERROR);
                continue;
            }
            self.send_text_file_with_references(file_id, &text.1.clone());
        }
        is_reference
    }

    /// If there is any reference to media files in the text file, request the media files
    fn send_referenced_files_requests(&mut self, text: &str, file_id: u8) {
        // First, look at the media files referenced inside the text file
        let first_line = text.lines().next();
        if first_line.is_none() {
            self.logger
                .log(&format!("Text file {file_id} is empty"), LogLevel::ERROR);
            return;
        }

        let first_line = first_line.unwrap(); // Impossible for a panic to happen, as it was just checked
        let has_reference = first_line.starts_with("ref=");

        // If the text file does not have a reference, skip it
        if !has_reference {
            self.logger.log(
                &format!("Text file {file_id} does not have a reference, sending to controller"),
                LogLevel::DEBUG,
            );
            // Send the text file to the sim controller
            let _res = self
                .sim_controller_sender
                .send(SimControllerResponseWrapper::Message(
                    SimControllerMessage::TextFileResponse(file_id, text.to_string()),
                ));
            return;
        }

        // Then, find a server of type media in the available servers
        let available_servers = self.get_available_servers().clone();
        let server_id = available_servers
            .iter()
            .find(|s| matches!(s.1, ServerType::Media));

        // If no media server is found, skip the text file
        if server_id.is_none() {
            self.logger.log(
                &format!("No media server found in available servers, cannot send media file references for text file {file_id}"),
                LogLevel::ERROR,
            );
            return;
        }

        let server_id = server_id.unwrap().0; // Impossible for a panic to happen, as it was just checked

        let references = first_line.split('=').collect::<Vec<&str>>()[1];
        let references = references.split(',').collect::<Vec<&str>>();

        // Add the file_id to the pending_referenced_files map
        self.pending_referenced_files
            .insert(file_id, HashSet::new());

        // Whether it needs to wait for at least one reference before sending, or if all references have already been obtained.
        let mut has_pending_references = false;
        // Request all the media files referenced in the text file
        for reference in references {
            let reference = reference.parse::<u8>();
            if reference.is_err() {
                self.logger.log(
                    &format!("Invalid reference in text file {file_id}"),
                    LogLevel::ERROR,
                );
                continue;
            }
            let reference = reference.unwrap(); // Impossible for a panic to happen, as it was just checked

            // Add the references to the references_files map
            self.references_files
                .entry(file_id)
                .or_default()
                .insert(reference);

            // If the media file is already obtained, skip it
            if self.obtained_media_files.keys().any(|k| *k == reference) {
                self.logger.log(
                    &format!("Media file {reference} already obtained, not sending request"),
                    LogLevel::DEBUG,
                );
                continue;
            }
            has_pending_references = true;

            // Add the references to the pending_referenced_files map
            self.pending_referenced_files
                .get_mut(&file_id)
                .unwrap_or(&mut HashSet::new())
                .insert(reference);

            // Request the media file
            self.request_media_file(reference, *server_id);
        }

        // If there are no pending references, send the text file to the sim controller with all the references
        if !has_pending_references {
            self.send_text_file_with_references(file_id, text);
        }
    }

    /// Send a text file with all the references to the controller
    fn send_text_file_with_references(&mut self, file_id: u8, text: &str) {
        self.pending_referenced_files.remove(&file_id);
        // Get the attached media files from the references, and get the obtained content
        let attached_media_files = self
            .references_files
            .remove(&file_id)
            .unwrap_or_default()
            .iter()
            .map(|file_id| {
                (
                    *file_id,
                    self.obtained_media_files
                        .get(file_id)
                        .unwrap_or(&Vec::new())
                        .clone(),
                )
            })
            .collect::<HashMap<u8, Vec<u8>>>();
        // If it's empty, something went wrong
        if attached_media_files.is_empty() {
            self.logger().log(
                &format!("The text file {file_id} does not have any attached media files"),
                LogLevel::ERROR,
            );
            return;
        }
        self.logger.log(
            &format!(
                "Sending text file {file_id} to sim controller, with attached media files: {:?}",
                attached_media_files.keys()
            ),
            LogLevel::DEBUG,
        );
        // Send to the simulation controller
        let _res = self
            .sim_controller_sender
            .send(SimControllerResponseWrapper::Message(
                SimControllerMessage::TextWithReferences(
                    file_id,
                    text.to_string(),
                    attached_media_files,
                ),
            ));
    }

    #[must_use]
    pub fn get_available_text_files(&self) -> &HashMap<NodeId, Vec<u8>> {
        &self.available_text_files
    }

    #[must_use]
    pub fn get_available_media_files(&self) -> &HashMap<NodeId, Vec<u8>> {
        &self.available_media_files
    }

    #[must_use]
    pub fn get_obtained_text_files(&self) -> &HashMap<(NodeId, u8), String> {
        &self.obtained_text_files
    }

    #[must_use]
    pub fn get_obtained_media_files(&mut self) -> &mut HashMap<u8, Vec<u8>> {
        &mut self.obtained_media_files
    }

    #[must_use]
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
                self.handle_browser_response(response, server_id);
            }
            // If the response is a ServerTypeResponse, handle it
            BrowserResponseWrapper::ServerType(server_response) => {
                let ServerTypeResponse::ServerType(server_response) = server_response;
                self.topology()
                    .set_node_type(server_id, format!("{server_response:?}"));
                self.logger.log(
                    &format!("Received server type: {server_response:?} from {server_id:?}"),
                    LogLevel::DEBUG,
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
                    ServerType::Chat => {
                        self.logger.log(
                            &format!(
                                "Server type 'Chat' not added to available servers: {server_response:?}"
                            ),
                            LogLevel::DEBUG,
                        );
                    }
                }

                // Send the server type response to the sim controller
                let response = SimControllerMessage::ServerTypeResponse(server_id, server_response);
                let _res = self
                    .sim_controller_sender
                    .send(SimControllerResponseWrapper::Message(response));
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
                self.logger.log(
                    &format!("COMMAND: Requesting file list from server {server_id}"),
                    LogLevel::DEBUG,
                );
                self.request_file_list(server_id);
            }
            // If the command is a request for a text file, send the request
            SimControllerCommand::RequestTextFile(file_id, server_id) => {
                self.logger.log(
                    &format!("COMMAND: Requesting text file {file_id} from server {server_id}"),
                    LogLevel::DEBUG,
                );
                self.request_text_file(file_id, server_id);
            }
            // If the command is a request for a media file, send the request
            SimControllerCommand::RequestMediaFile(file_id, server_id) => {
                self.logger.log(
                    &format!("COMMAND: Requesting media file {file_id} from server {server_id}"),
                    LogLevel::DEBUG,
                );
                self.request_media_file(file_id, server_id);
            }
            // If the command is a flood request, send the request
            SimControllerCommand::FloodRequest => {
                self.logger
                    .log("COMMAND: Sending flood request", LogLevel::DEBUG);
                self.send_flood_request();
            }
            // If the command is a topology request, send the topology
            SimControllerCommand::Topology => {
                let topology = self.topology.clone();
                self.logger
                    .log("COMMAND: Sending topology", LogLevel::DEBUG);
                let response = SimControllerMessage::TopologyResponse(topology);
                let _res = self
                    .sim_controller_sender
                    .send(SimControllerResponseWrapper::Message(response));
            }
            // If the command is to add a neighbor, add the neighbor to the map and the topology
            SimControllerCommand::AddSender(sender_id, sender_channel) => {
                self.logger.log(
                    &format!("COMMAND: Adding sender {sender_id}"),
                    LogLevel::DEBUG,
                );
                self.senders.insert(sender_id, sender_channel);
                self.topology.add_node(sender_id);
                self.topology.set_node_type(sender_id, "drone".to_string());
                self.topology.add_edge(self.client_id, sender_id);
                // Send a flood request to the new neighbor
                self.send_flood_request();
            }
            // If the command is to remove a neighbor, remove the neighbor from the map and the topology
            SimControllerCommand::RemoveSender(sender_id) => {
                self.logger.log(
                    &format!("COMMAND: Removing sender {sender_id}"),
                    LogLevel::DEBUG,
                );
                self.topology.remove_edges(self.client_id, sender_id);
                self.senders.remove(&sender_id);
            }
            // If the command wants the servers known by the client, send the known servers
            SimControllerCommand::KnownServers => {
                self.logger.log(
                    &format!(
                        "COMMAND: Sending known servers ({:?})",
                        self.available_servers
                    ),
                    LogLevel::DEBUG,
                );
                // Check the server types, if any are unknown (Server), request the type
                let node_types = self.topology.get_node_types().clone();
                for (server_id, server_type) in node_types {
                    if server_type == "server" {
                        self.send_server_type_request(server_id);
                    }
                }
                // Then, send the response
                let known_servers = self.available_servers.clone();
                let response = SimControllerMessage::KnownServers(known_servers);
                let _res = self
                    .sim_controller_sender
                    .send(SimControllerResponseWrapper::Message(response));
            }
            SimControllerCommand::RequestServerType(server_id) => {
                self.logger.log(
                    &format!("COMMAND: Requesting server type from server {server_id}"),
                    LogLevel::DEBUG,
                );
                self.send_server_type_request(server_id);
            }
            // The simulation controller wants the client to shut down
            SimControllerCommand::Shutdown => {
                self.logger.log("COMMAND: Shutting down", LogLevel::DEBUG);
                process::exit(0);
            }
            // Commands related to the Chat Client
            _ => {
                self.logger.log(
                    &format!("COMMAND: Unrecognized command: {command:?}"),
                    LogLevel::ERROR,
                );
            }
        }
    }

    fn acked_packets(&mut self) -> &mut HashMap<u64, Vec<bool>> {
        &mut self.acked_packets
    }

    fn send_server_type_request(&mut self, server_id: NodeId) {
        self.logger.log(
            &format!("Sending server type request to server {server_id}"),
            LogLevel::DEBUG,
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

    fn logger(&self) -> &Logger {
        &self.logger
    }
}
