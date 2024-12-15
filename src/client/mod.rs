use std::collections::HashMap;

use crate::message::{
    ChatRequest, ChatResponse, DroneSend, Message, Request, Response, ServerType,
    ServerTypeRequest, ServerTypeResponse,
};
use crossbeam::select;
use crossbeam_channel::{Receiver, Sender};
use wg_2024::{
    network::*,
    packet::{Fragment, Packet, PacketType},
};

pub trait Client {
    type RequestType: Request;
    type ResponseType: Response;

    fn client_id(&self) -> u8;
    fn senders(&self) -> &HashMap<u8, Sender<Packet>>;
    fn receiver(&self) -> &Receiver<Packet>;
    fn received_fragments(&mut self) -> &mut HashMap<u64, Vec<Fragment>>;

    fn compose_message(
        &self,
        source_id: NodeId,
        session_id: u64,
        raw_content: String,
    ) -> Result<Message<Self::ResponseType>, String> {
        let content = Self::ResponseType::from_string(raw_content)?;
        Ok(Message {
            session_id,
            source_id,
            content,
        })
    }

    fn on_response_arrived(&mut self, source_id: NodeId, session_id: u64, raw_content: String) {
        match self.compose_message(source_id, session_id, raw_content) {
            Ok(message) => {
                let response = self.handle_response(message.content);
                // TODO: send ACK
            }
            Err(str) => panic!("{}", str),
        }
    }

    fn run(&mut self) {
        loop {
            select! {
                recv(self.receiver()) -> packet => {
                    match packet {
                        Ok(packet) => {
                            match packet.pack_type {
                                PacketType::MsgFragment(fragment) => {
                                    if let Some(message) = self.add_fragment(fragment, packet.session_id) {
                                        let message_str = String::from_utf8_lossy(&message);
                                        self.on_response_arrived(0, packet.session_id, message_str.to_string());
                                    }
                                }
                                PacketType::FloodResponse(flood_response) => {
                                    println!(
                                        "Client {} received FloodResponse: {:?}",
                                        self.client_id(), flood_response
                                    );
                                }
                                _ => {
                                    println!("Client {} received an unsupported packet type", self.client_id());
                                }
                            }
                        }
                        Err(err) => {
                            eprintln!("Client {}: Error receiving packet: {:?}", self.client_id(), err);
                        }
                    }
                }
            }
        }
    }

    fn add_fragment(&mut self, fragment: Fragment, session_id: u64) -> Option<Vec<u8>> {
        let fragments = self
            .received_fragments()
            .entry(session_id)
            .or_insert_with(Vec::new);

        fragments.push(fragment);

        if let Some(total_fragments) = fragments.first().map(|f| f.total_n_fragments) {
            if fragments.len() == total_fragments as usize {
                return Some(self.reassemble_message(session_id));
            }
        }
        None
    }

    fn reassemble_message(&mut self, session_id: u64) -> Vec<u8> {
        if let Some(mut fragments) = self.received_fragments().remove(&session_id) {
            fragments.sort_by_key(|f| f.fragment_index);

            let mut message = Vec::new();
            for fragment in fragments {
                message.extend_from_slice(&fragment.data[..fragment.length as usize]);
            }
            return message;
        }
        Vec::new()
    }

    fn handle_response(&mut self, response: Self::ResponseType);

    // TODO: method to send message
}