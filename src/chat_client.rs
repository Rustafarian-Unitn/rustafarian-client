use std::collections::HashMap;

use crate::{assembler::{self, assembler::Assembler, disassembler::Disassembler}, client::Client, message::{
    ChatRequest, ChatResponse, Message, ServerType,
}, topology::Topology};
use crossbeam_channel::{Receiver, Sender};
use wg_2024::{network::NodeId, packet::{Fragment, Packet, PacketType}};

pub struct ChatClient {
    client_id: u8,
    senders: HashMap<u8, Sender<Packet>>,
    receiver: Receiver<Packet>,
    topology: Topology,
    sim_controller_receiver: Receiver<Message<ChatResponse>>,
    sent_packets: HashMap<u64, Packet>,
    available_clients: Vec<NodeId>,
    assembler: Assembler,
    deassembler: Disassembler
}

impl ChatClient {
    pub fn new(
        client_id: u8,
        senders: HashMap<u8, Sender<Packet>>,
        receiver: Receiver<Packet>,
        sim_controller_receiver: Receiver<Message<ChatResponse>>
    ) -> Self {
        ChatClient {
            client_id,
            senders,
            receiver,
            topology: Topology::new(),
            sim_controller_receiver,
            sent_packets: HashMap::new(),
            available_clients: Vec::new(),
            assembler: Assembler::new(),
            deassembler: Disassembler::new()
        }
    }

    /// Send a 'register' message to a server
    pub fn register(&mut self, server_id: NodeId) -> () {
        let request = ChatRequest::Register(self.client_id);
        let request_json = serde_json::to_string(&request).unwrap();
        let fragments = self.deassembler.disassemble_message(request_json.as_bytes().to_vec(), 0);
        let session_id = rand::random();
        let routing_header = self.topology.get_routing_header(self.client_id(), server_id);
        let first_hop_id = routing_header.current_hop().unwrap();
        for fragment in fragments {
            let packet = Packet::new_fragment(routing_header.clone(), session_id, fragment);
            self.senders.get_mut(&first_hop_id).unwrap().send(packet).unwrap();
        }
    }
    
    /// Get the list of available clients in the chat server
    pub fn get_client_list(&mut self) -> &mut Vec<NodeId> {
        &mut self.available_clients
    }
}

impl Client for ChatClient {
    type RequestType = ChatRequest;
    type ResponseType = ChatResponse;

    fn client_id(&self) -> u8 {
        self.client_id
    }

    fn senders(&self) -> &HashMap<u8, Sender<Packet>> {
        &self.senders
    }

    fn receiver(&self) -> &Receiver<Packet> {
        &self.receiver
    }

    fn topology(&mut self) -> &mut crate::topology::Topology {
        &mut self.topology
    }

    fn handle_response(&mut self, response: Self::ResponseType) {
        match response {
            ChatResponse::ClientList(client_list) => {
                println!("Client list: {:?}", client_list);
                self.available_clients = client_list;
            }
            ChatResponse::MessageFrom { from, message } => {
                println!("Message from {}: {:?}", from, message);
            }
            ChatResponse::MessageSent => {
                println!("Message sent");
            }
        }
    }
    
    fn sim_controller_receiver(&self) -> &Receiver<Message<Self::ResponseType>> {
        &self.sim_controller_receiver
    }
    
    fn assembler(&mut self) -> &mut crate::assembler::assembler::Assembler {
        &mut self.assembler
    }
    
    fn deassembler(&mut self) -> &mut crate::assembler::disassembler::Disassembler {
        &mut self.deassembler
    }
    
    fn sent_packets(&mut self) -> &mut HashMap<u64, Packet> {
        &mut self.sent_packets
    }

}