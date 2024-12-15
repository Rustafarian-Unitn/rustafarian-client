use std::collections::HashMap;

use crate::{client::Client, message::{
    ChatRequest, ChatResponse,
}};
use crossbeam_channel::{Receiver, Sender};
use wg_2024::packet::{Fragment, Packet};

pub struct ChatClient {
    client_id: u8,
    senders: HashMap<u8, Sender<Packet>>,
    receiver: Receiver<Packet>,
    received_fragment:HashMap<u64, Vec<Fragment>>
}

impl ChatClient {
    pub fn new(
        client_id: u8,
        senders: HashMap<u8, Sender<Packet>>,
        receiver: Receiver<Packet>,
    ) -> Self {
        ChatClient {
            client_id,
            senders,
            receiver,
            received_fragment: HashMap::new()
        }
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

    fn received_fragments(&mut self) -> &mut HashMap<u64, Vec<Fragment>> {
        &mut self.received_fragment
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

}