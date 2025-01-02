use core::str;
use std::collections::HashMap;

use crate::client::Client;
use rustafarian_shared::assembler::{assembler::Assembler, disassembler::Disassembler};
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
    flood_in_progress: bool,

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
            flood_in_progress: false,

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
        println!(
            "Client {} registering to server {}",
            self.client_id, server_id
        );
        let request = ChatRequestWrapper::Chat(ChatRequest::Register(self.client_id));
        let request_json = serde_json::to_string(&request).unwrap();
        self.send_message(server_id, request_json);
    }

    /// Send a chat message to another client
    pub fn send_chat_message(&mut self, server_id: NodeId, to: NodeId, message: String) {
        println!(
            "Client {} sending message to {} using {}",
            self.client_id, to, server_id
        );
        let chat_message = ChatRequestWrapper::Chat(ChatRequest::SendMessage {
            from: self.client_id,
            to,
            message,
        });
        let chat_message_json = serde_json::to_string(&chat_message).unwrap();

        self.send_message(server_id, chat_message_json.clone());

        // Notify the controller that the message was sent
        let _res = self
            .sim_controller_sender
            .send(SimControllerResponseWrapper::Event(
                SimControllerEvent::MessageSent(server_id, to, chat_message_json),
            ));
    }

    /// Send a ClientList request to a server, asking for the clients registered to it
    pub fn send_client_list_req(&mut self, server_id: NodeId) {
        println!(
            "Client {} sending client list request to {}",
            self.client_id, server_id
        );

        let request = ChatRequestWrapper::Chat(ChatRequest::ClientList);
        let request_json = serde_json::to_string(&request).unwrap();
        self.send_message(server_id, request_json);
    }

    /// Handle a chat response from a server
    fn handle_chat_response(&mut self, response: ChatResponse, server_id: NodeId) {
        match response {
            // If the response is a client list, add them to the available_clients for that server
            ChatResponse::ClientList(client_list) => {
                println!("Client list: {:?}", client_list);
                self.available_clients
                    .insert(server_id, client_list.clone());
                // Send info to the controller
                let response = SimControllerMessage::ClientListResponse(server_id, client_list);
                self.sim_controller_sender
                    .send(SimControllerResponseWrapper::Message(response))
                    .unwrap();
            }
            // If the response is a message, print it, and send to the controller
            ChatResponse::MessageFrom { from, message } => {
                let s = match str::from_utf8(message.as_ref()) {
                    Ok(v) => v,
                    Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
                };
                println!("Message from {}: {:?}", from, s);
                // Send the message to the controller
                let _res = self
                    .sim_controller_sender
                    .send(SimControllerResponseWrapper::Message(
                        SimControllerMessage::MessageReceived(server_id, from, s.to_string()),
                    ));
            }
            // The message was sent correctly
            ChatResponse::MessageSent => {
                println!("Message sent");
            }
            // The client was registered correctly
            ChatResponse::ClientRegistered => {
                println!("Client registered");
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
                println!("Server response: {:?}", server_response);
                let ServerTypeResponse::ServerType(server_response) = server_response;
                self.topology()
                    .set_node_type(server_id, format!("{:?}", server_response));
                // If it's a chat server, add it to the available servers (as a key of available_clients)
                if let ServerType::Chat = server_response {
                    self.available_clients.insert(server_id, vec![]);
                }

                // send the server type response to the sim controller
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

    /// Handle the commands sent by the controller
    fn handle_controller_commands(&mut self, command: SimControllerCommand) {
        match command {
            // Send a message to a client
            SimControllerCommand::SendMessage(message, server_id, to) => {
                println!("COMMAND: Sending message to {} using {}", to, server_id);
                self.send_chat_message(server_id, to, message);
            }
            // Register to a server
            SimControllerCommand::Register(server_id) => {
                println!("COMMAND: Registering to server {}", server_id);
                self.register(server_id);
            }
            // Get the list of clients registered to a server
            SimControllerCommand::ClientList(server_id) => {
                println!("COMMAND: Getting client list from server {}", server_id);
                self.send_client_list_req(server_id);
            }
            // Send a flood request
            SimControllerCommand::FloodRequest => {
                println!("COMMAND: Sending flood request");
                self.send_flood_request();
            }
            // Get the topology as seen by the client
            SimControllerCommand::Topology => {
                println!("COMMAND: Sending topology");
                let topology = self.topology.clone();
                let response = SimControllerMessage::TopologyResponse(topology);
                self.sim_controller_sender
                    .send(SimControllerResponseWrapper::Message(response))
                    .unwrap();
            }
            // Get the list of servers the client is registered to
            SimControllerCommand::RegisteredServers => {
                println!("COMMAND: Getting registered servers");
                let response = SimControllerMessage::RegisteredServersResponse(
                    self.registered_servers.clone(),
                );
                self.sim_controller_sender
                    .send(SimControllerResponseWrapper::Message(response))
                    .unwrap();
            }
            // Get the list of known servers
            SimControllerCommand::KnownServers => {
                println!("COMMAND: Getting known servers");
                // All the registered servers are of type chat
                let mut map = HashMap::new();
                for (server_id, _) in self.available_clients.iter() {
                    map.insert(*server_id, ServerType::Chat);
                }
                let response = SimControllerMessage::KnownServers(map);
                self.sim_controller_sender
                    .send(SimControllerResponseWrapper::Message(response))
                    .unwrap();
            }
            // Add a neighbor
            SimControllerCommand::AddSender(sender_id, sender_channel) => {
                println!("COMMAND: Adding sender {}", sender_id);
                self.senders.insert(sender_id, sender_channel);
                self.topology.add_node(sender_id);
                self.topology.add_edge(self.client_id, sender_id);
            }
            // Remove a neighbor
            SimControllerCommand::RemoveSender(sender_id) => {
                println!("COMMAND: Removing sender {}", sender_id);
                self.senders.remove(&sender_id);
                self.topology.remove_node(sender_id);
            }
            _ => {}
        }
    }

    fn acked_packets(&mut self) -> &mut HashMap<u64, Vec<bool>> {
        &mut self.acked_packets
    }

    /// Send a ServerType request to a server
    fn send_server_type_request(&mut self, server_id: NodeId) {
        println!(
            "Client {} sending server type request to {}",
            self.client_id, server_id
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

    fn flood_in_progress(&mut self) -> &mut bool {
        &mut self.flood_in_progress
    }
}
