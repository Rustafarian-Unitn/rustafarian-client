use core::str;
use std::collections::HashMap;

use crate::client::Client;
use rustafarian_shared::assembler::{assembler::Assembler, disassembler::Disassembler};
use rustafarian_shared::logger::{LogLevel, Logger};
use rustafarian_shared::messages::chat_messages::{
    ChatRequest, ChatRequestWrapper, ChatResponse, ChatResponseWrapper,
};
use rustafarian_shared::messages::commander_messages::{
    SimControllerCommand, SimControllerEvent, SimControllerMessage, SimControllerResponseWrapper,
};
use rustafarian_shared::messages::general_messages::{
    DroneSend, ServerType, ServerTypeRequest, ServerTypeResponse,
};
use rustafarian_shared::topology::Topology;

use crossbeam_channel::{Receiver, Sender};
use wg_2024::{network::NodeId, packet::Packet};

pub struct ChatClient {
    // General data for Client
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

    // Chat-specific data
    /// Key: server_id, value: list of client ids
    available_clients: HashMap<NodeId, Vec<NodeId>>,
    /// List of servers the client is registered to
    registered_servers: Vec<NodeId>,
}

impl ChatClient {
    pub fn new(
        client_id: u8,
        senders: HashMap<u8, Sender<Packet>>,
        receiver: Receiver<Packet>,
        sim_controller_receiver: Receiver<SimControllerCommand>,
        sim_controller_sender: Sender<SimControllerResponseWrapper>,
        debug: bool,
    ) -> Self {
        ChatClient {
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
            logger: Logger::new("ChatClient".to_string(), client_id, debug),

            available_clients: HashMap::new(),
            registered_servers: vec![],
        }
    }

    /// Get the list of available clients in the chat server
    pub fn get_client_list(&mut self) -> &mut HashMap<NodeId, Vec<NodeId>> {
        &mut self.available_clients
    }

    /// Send a 'register' message to a server
    pub fn register(&mut self, server_id: NodeId) {
        self.logger.log(
            &format!(
                "Client {} registering to server {}",
                self.client_id, server_id
            ),
            LogLevel::DEBUG,
        );
        let request = ChatRequestWrapper::Chat(ChatRequest::Register(self.client_id));
        let request_json = serde_json::to_string(&request).unwrap_or("".to_string());
        self.send_message(server_id, request_json);
    }

    /// Send a chat message to another client
    pub fn send_chat_message(&mut self, server_id: NodeId, to: NodeId, message: String) {
        self.logger.log(
            &format!("Sending message to {} using {}", to, server_id),
            LogLevel::DEBUG,
        );
        let chat_message = ChatRequestWrapper::Chat(ChatRequest::SendMessage {
            from: self.client_id,
            to,
            message,
        });
        let chat_message_json = serde_json::to_string(&chat_message).unwrap_or("".to_string());

        self.send_message(server_id, chat_message_json.clone());

        // Notify the controller that the message was sent
        let _res = self
            .sim_controller_sender
            .send(SimControllerResponseWrapper::Event(
                SimControllerEvent::ChatMessageSent(server_id, to, chat_message_json),
            ));
    }

    /// Send a ClientList request to a server, asking for the clients registered to it
    pub fn send_client_list_req(&mut self, server_id: NodeId) {
        self.logger.log(
            &format!("Sending client list request to {}", server_id),
            LogLevel::DEBUG,
        );

        let request = ChatRequestWrapper::Chat(ChatRequest::ClientList);
        let request_json = serde_json::to_string(&request).unwrap_or("".to_string());
        self.send_message(server_id, request_json);
    }

    /// Handle a chat response from a server
    fn handle_chat_response(&mut self, response: ChatResponse, server_id: NodeId) {
        match response {
            // If the response is a client list, add them to the available_clients for that server
            ChatResponse::ClientList(client_list) => {
                self.logger.log(
                    &format!("Received client list: {:?} from {}", client_list, server_id),
                    LogLevel::DEBUG,
                );
                self.available_clients
                    .insert(server_id, client_list.clone());
                // Send info to the controller
                let response = SimControllerMessage::ClientListResponse(server_id, client_list);
                let _res = self
                    .sim_controller_sender
                    .send(SimControllerResponseWrapper::Message(response));
            }
            // If the response is a message, print it, and send to the controller
            ChatResponse::MessageFrom { from, message } => {
                let s = match str::from_utf8(message.as_ref()) {
                    Ok(v) => v,
                    Err(e) => {
                        self.logger()
                            .log(&format!("Invalid UTF-8 sequence: {}", e), LogLevel::ERROR);
                        "Invalid UTF-8 sequence"
                    }
                };
                self.logger.log(
                    &format!("Received message from {}: {}", from, s),
                    LogLevel::DEBUG,
                );
                // Send the message to the controller
                let _res = self
                    .sim_controller_sender
                    .send(SimControllerResponseWrapper::Message(
                        SimControllerMessage::MessageReceived(server_id, from, s.to_string()),
                    ));
            }
            // The message was sent correctly
            ChatResponse::MessageSent => {
                self.logger
                    .log(&format!("Message sent from {}", server_id), LogLevel::DEBUG);
            }
            // The client was registered correctly
            ChatResponse::ClientRegistered => {
                self.logger
                    .log(&format!("Registered to {}", server_id), LogLevel::DEBUG);
                // Add the server to the list of registered servers
                self.registered_servers.push(server_id);
            }
        };
    }

    /// Get the servers the client is registered to
    pub fn get_registered_servers(&mut self) -> &mut Vec<NodeId> {
        &mut self.registered_servers
    }

    /// Get the available servers
    pub fn get_available_clients(&mut self) -> &mut HashMap<NodeId, Vec<NodeId>> {
        &mut self.available_clients
    }
}

/// Implement default methods for the Client
impl Client for ChatClient {
    type RequestType = ChatRequestWrapper;
    type ResponseType = ChatResponseWrapper;

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
            ChatResponseWrapper::Chat(response) => self.handle_chat_response(response, server_id),
            ChatResponseWrapper::ServerType(server_response) => {
                let ServerTypeResponse::ServerType(server_response) = server_response;
                self.logger.log(
                    &format!(
                        "Received server type: {:?} from {:?}",
                        server_response, server_id
                    ),
                    LogLevel::DEBUG,
                );
                self.topology()
                    .set_node_type(server_id, format!("{:?}", server_response));
                // If it's a chat server, add it to the available servers (as a key of available_clients)
                if let ServerType::Chat = server_response {
                    self.available_clients.insert(server_id, vec![]);
                }

                // send the server type response to the sim controller
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

    /// Handle the commands sent by the controller
    fn handle_controller_commands(&mut self, command: SimControllerCommand) {
        match command {
            // Send a message to a client
            SimControllerCommand::SendMessage(message, server_id, to) => {
                self.logger.log(
                    &format!("COMMAND: Sending message to {} using {}", to, server_id),
                    LogLevel::DEBUG,
                );
                self.send_chat_message(server_id, to, message);
            }
            // Register to a server
            SimControllerCommand::Register(server_id) => {
                self.logger.log(
                    &format!("COMMAND: Registering to server {}", server_id),
                    LogLevel::DEBUG,
                );
                self.register(server_id);
            }
            // Get the list of clients registered to a server
            SimControllerCommand::ClientList(server_id) => {
                self.logger.log(
                    &format!("COMMAND: Getting client list from server {}", server_id),
                    LogLevel::DEBUG,
                );
                self.send_client_list_req(server_id);
            }
            // Send a flood request
            SimControllerCommand::FloodRequest => {
                self.logger
                    .log("COMMAND: Sending flood request", LogLevel::DEBUG);
                self.send_flood_request();
            }
            // Get the topology as seen by the client
            SimControllerCommand::Topology => {
                self.logger
                    .log("COMMAND: Sending topology", LogLevel::DEBUG);
                let topology = self.topology.clone();
                let response = SimControllerMessage::TopologyResponse(topology);
                let _res = self
                    .sim_controller_sender
                    .send(SimControllerResponseWrapper::Message(response));
            }
            // Get the list of servers the client is registered to
            SimControllerCommand::RegisteredServers => {
                self.logger
                    .log("COMMAND: Getting registered servers", LogLevel::DEBUG);
                let response = SimControllerMessage::RegisteredServersResponse(
                    self.registered_servers.clone(),
                );
                let _res = self
                    .sim_controller_sender
                    .send(SimControllerResponseWrapper::Message(response));
            }
            // Get the list of known servers
            SimControllerCommand::KnownServers => {
                self.logger
                    .log("COMMAND: Getting known servers", LogLevel::DEBUG);
                // All the registered servers are of type chat
                let mut map = HashMap::new();
                for (server_id, _) in self.available_clients.iter() {
                    map.insert(*server_id, ServerType::Chat);
                }
                // Check the server types, if any are unknown (Server), request the type
                let node_types = self.topology.get_node_types().clone();
                for (server_id, server_type) in node_types {
                    if server_type == "Server" {
                        self.send_server_type_request(server_id);
                    }
                }
                // Then, send the response
                let response = SimControllerMessage::KnownServers(map);
                let _res = self
                    .sim_controller_sender
                    .send(SimControllerResponseWrapper::Message(response));
            }
            // Add a neighbor
            SimControllerCommand::AddSender(sender_id, sender_channel) => {
                self.logger.log(
                    &format!("COMMAND: Adding sender {}", sender_id),
                    LogLevel::DEBUG,
                );
                self.senders.insert(sender_id, sender_channel);
                self.topology.add_node(sender_id);
                self.topology.add_edge(self.client_id, sender_id);
                // Send a flood request to the new neighbor
                self.send_flood_request();
            }
            // Remove a neighbor
            SimControllerCommand::RemoveSender(sender_id) => {
                self.logger.log(
                    &format!("COMMAND: Removing sender {}", sender_id),
                    LogLevel::DEBUG,
                );
                self.senders.remove(&sender_id);
                self.topology.remove_edges(self.client_id, sender_id);
            }
            SimControllerCommand::RequestServerType(server_id) => {
                self.logger.log(
                    &format!("COMMAND: Requesting server type from server {}", server_id),
                    LogLevel::DEBUG,
                );
                self.send_server_type_request(server_id);
            }
            _ => {}
        }
    }

    fn acked_packets(&mut self) -> &mut HashMap<u64, Vec<bool>> {
        &mut self.acked_packets
    }

    /// Send a ServerType request to a server
    fn send_server_type_request(&mut self, server_id: NodeId) {
        self.logger.log(
            &format!("Sending server type request to {}", server_id),
            LogLevel::DEBUG,
        );
        let request = ServerTypeRequest::ServerType;
        let request_wrapped = ChatRequestWrapper::ServerType(request);
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
