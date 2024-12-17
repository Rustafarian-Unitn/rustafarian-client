use std::collections::HashMap;

use crate::{assembler::{self, assembler::Assembler, deassembler::Deassembler}, client::Client, message::{
    ChatRequest, ChatResponse, Message,
}, topology::Topology};
use crossbeam_channel::{Receiver, Sender};
use wg_2024::packet::{Fragment, Packet, PacketType};

pub struct ChatClient {
    client_id: u8,
    senders: HashMap<u8, Sender<Packet>>,
    receiver: Receiver<Packet>,
    received_fragment:HashMap<u64, Vec<Fragment>>,
    topology: Topology,
    sim_controller_receiver: Receiver<Message<ChatResponse>>,
    sent_packets: HashMap<u64, Packet>,
    assembler: Assembler,
    deassembler: Deassembler
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
            received_fragment: HashMap::new(),
            topology: Topology::new(),
            sim_controller_receiver,
            sent_packets: HashMap::new(),
            assembler: Assembler::new(),
            deassembler: Deassembler::new()
        }
    }

    pub fn register(&mut self) -> () {
        let request = ChatRequest::Register(self.client_id);
        let request_json = serde_json::to_string(&request).unwrap();
        let fragments = self.deassembler.add_message(request_json.as_bytes().to_vec(), 0);
        // TODO
        todo!();
    }
    
    pub fn get_client_list(&self) -> () {
        // TODO
        todo!()
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
    
    fn deassembler(&mut self) -> &mut crate::assembler::deassembler::Deassembler {
        &mut self.deassembler
    }
    
    fn sent_packets(&mut self) -> &mut HashMap<u64, Packet> {
        &mut self.sent_packets
    }

}