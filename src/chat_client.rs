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
    packets_to_send: HashMap<u64, Packet>,

    // Chat-specific data
    available_clients: HashMap<NodeId, Vec<NodeId>>, // Key: server_id, value: list of client ids
    registered_servers: Vec<NodeId>,                 // List of servers the client is registered to
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
        let request = ChatRequestWrapper::Chat(ChatRequest::Register(self.client_id));
        let request_json = serde_json::to_string(&request).unwrap();
        self.send_message(server_id, request_json);
    }

    pub fn send_chat_message(&mut self, server_id: NodeId, to: NodeId, message: String) {
        let chat_message = ChatRequestWrapper::Chat(ChatRequest::SendMessage {
            from: self.client_id,
            to,
            message,
        });
        let chat_message_json = serde_json::to_string(&chat_message).unwrap();

        self.send_message(server_id, chat_message_json.clone());

        let _res = self
            .sim_controller_sender
            .send(SimControllerResponseWrapper::Event(
                SimControllerEvent::MessageSent(server_id, to, chat_message_json),
            ));
    }

    pub fn send_client_list_req(&mut self, server_id: NodeId) {
        let request = ChatRequestWrapper::Chat(ChatRequest::ClientList);
        let request_json = serde_json::to_string(&request).unwrap();
        self.send_message(server_id, request_json);
    }

    fn handle_chat_response(&mut self, response: ChatResponse, server_id: NodeId) {
        match response {
            ChatResponse::ClientList(client_list) => {
                println!("Client list: {:?}", client_list);
                self.available_clients.insert(server_id, client_list);
            }
            ChatResponse::MessageFrom { from, message } => {
                let s = match str::from_utf8(message.as_ref()) {
                    Ok(v) => v,
                    Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
                };
                println!("Message from {}: {:?}", from, s);
                let _res = self
                    .sim_controller_sender
                    .send(SimControllerResponseWrapper::Message(
                        SimControllerMessage::MessageReceived(server_id, from, s.to_string()),
                    ));
            }
            ChatResponse::MessageSent => {
                println!("Message sent");
            }
            ChatResponse::ClientRegistered => {
                println!("Client registered");
                self.registered_servers.push(server_id);
            }
        };
    }
}

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

    fn handle_controller_commands(&mut self, command: SimControllerCommand) {
        match command {
            SimControllerCommand::SendMessage(message, server_id, to) => {
                println!("Sending message to {} using {}", to, server_id);
                self.send_chat_message(server_id, to, message);
            }
            SimControllerCommand::Register(server_id) => {
                self.register(server_id);
            }
            SimControllerCommand::ClientList(server_id) => {
                self.send_client_list_req(server_id);
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
            SimControllerCommand::RegisteredServers => {
                let response = SimControllerMessage::RegisteredServersResponse(
                    self.registered_servers.clone(),
                );
                self.sim_controller_sender
                    .send(SimControllerResponseWrapper::Message(response))
                    .unwrap();
            }
            SimControllerCommand::KnownServers => {
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
            SimControllerCommand::AddSender(sender_id, sender_channel) => {
                self.senders.insert(sender_id, sender_channel);
            }
            SimControllerCommand::RemoveSender(sender_id) => {
                self.senders.remove(&sender_id);
            }
            _ => {}
        }
    }

    fn acked_packets(&mut self) -> &mut HashMap<u64, Vec<bool>> {
        &mut self.acked_packets
    }

    fn send_server_type_request(&mut self, server_id: NodeId) {
        let request = ServerTypeRequest::ServerType;
        let request_wrapped = ChatRequestWrapper::ServerType(request);
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
