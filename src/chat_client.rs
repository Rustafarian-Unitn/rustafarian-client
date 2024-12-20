use std::collections::HashMap;

use crate::client::Client;
use rustafarian_shared::assembler::{assembler::Assembler, disassembler::Disassembler};
use rustafarian_shared::messages::chat_messages::{ChatRequest, ChatResponse, ChatResponseWrapper};
use rustafarian_shared::messages::commander_messages::{
    SimControllerCommand, SimControllerMessage, SimControllerResponseWrapper,
};
use rustafarian_shared::topology::Topology;

use crossbeam_channel::{Receiver, Sender};
use wg_2024::{network::NodeId, packet::Packet};

pub struct ChatClient {
    client_id: u8,
    senders: HashMap<u8, Sender<Packet>>,
    receiver: Receiver<Packet>,
    topology: Topology,
    sim_controller_receiver: Receiver<SimControllerCommand>,
    sim_controller_sender: Sender<SimControllerResponseWrapper>,
    sent_packets: HashMap<u64, Vec<Packet>>,
    acked_packets: HashMap<u64, usize>,
    available_clients: HashMap<NodeId, Vec<NodeId>>, // Key: server_id, value: list of client ids
    assembler: Assembler,
    disassembler: Disassembler,
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
            available_clients: HashMap::new(),
            assembler: Assembler::new(),
            disassembler: Disassembler::new(),
        }
    }

    /// Get the list of available clients in the chat server
    pub fn get_client_list(&mut self) -> &mut HashMap<NodeId, Vec<NodeId>> {
        &mut self.available_clients
    }

    /// Send a 'register' message to a server
    pub fn register(&mut self, server_id: NodeId) -> () {
        let request = ChatRequest::Register(self.client_id);
        let request_json = serde_json::to_string(&request).unwrap();
        self.send_message(server_id, request_json);
    }

    pub fn send_chat_message(&mut self, server_id: NodeId, message: String) {
        let chat_message = ChatRequest::SendMessage {
            from: self.client_id,
            to: server_id,
            message,
        };
        let chat_message_json = serde_json::to_string(&chat_message).unwrap();

        self.send_message(server_id, chat_message_json);
    }

    pub fn send_client_list_req(&mut self, server_id: NodeId) {
        let request = ChatRequest::ClientList;
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
                println!("Message from {}: {:?}", from, message);
            }
            ChatResponse::MessageSent => {
                println!("Message sent");
            }
        };
    }
}

impl Client for ChatClient {
    type RequestType = ChatRequest;
    type ResponseType = ChatResponseWrapper;
    type SimControllerMessage = SimControllerResponseWrapper;
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
            ChatResponseWrapper::Chat(response) => self.handle_chat_response(response, server_id),
            ChatResponseWrapper::ServerType(server_response) => {
                println!("Server response: {:?}", server_response)
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
            SimControllerCommand::SendMessage(message, server_id, to) => {
                self.send_chat_message(to, message);
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
            _ => {}
        }
    }
    
    fn acked_packets_count(&mut self) -> &mut HashMap<u64, usize> {
        &mut self.acked_packets
    }
}
