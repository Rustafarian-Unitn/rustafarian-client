use std::collections::HashMap;

use crate::{message::{
    ChatRequest, ChatResponse, DroneSend, Message, Request, Response, ServerType,
    ServerTypeRequest, ServerTypeResponse,
}, topology::Topology};
use crossbeam::select;
use crossbeam_channel::{Receiver, Sender};
use wg_2024::{
    network::*,
    packet::{FloodRequest, Fragment, Packet, PacketType},
};
pub const FRAGMENT_DSIZE: usize = 128;

pub trait Client {
    type RequestType: Request;
    type ResponseType: Response;

    fn client_id(&self) -> u8;
    fn senders(&self) -> &HashMap<u8, Sender<Packet>>;
    fn receiver(&self) -> &Receiver<Packet>;
    fn received_fragments(&mut self) -> &mut HashMap<u64, Vec<Fragment>>;
    fn topology(&mut self) -> &mut Topology;

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

    fn deassemble_message(&mut self, message: Vec<u8>, session_id: u64) -> Vec<Fragment> {
        let mut fragments = Vec::<Fragment>::new();
        //ceil rounds the decimal number to the next whole number
        let total_fragments = (message.len() as f64 / FRAGMENT_DSIZE as f64).ceil() as u64;

        // Break the message into fragments (chunks) and iter over it
        for (i, chunk) in message.chunks(FRAGMENT_DSIZE).enumerate() {
            //divide the message in chunks
            let mut data = [0u8; FRAGMENT_DSIZE];
            let length = chunk.len() as u8; // chunk length

            // copy chucnk into data
            data[..length as usize].copy_from_slice(chunk);

            // create fragment and add it to list
            let fragment = Fragment {
                fragment_index: i as u64,
                total_n_fragments: total_fragments,
                length,
                data,
            };

            fragments.push(fragment);
        }

        fragments
    }

    fn handle_response(&mut self, response: Self::ResponseType);

    fn send_packet(&mut self, client_id: u8, message: Packet) {
        match self.senders().get(&client_id) {
            Some(sender) => {
                sender.send(message).unwrap();
            }
            None => {
                eprintln!("Client {}: No sender found for client {}", self.client_id(), client_id);
            }
        }
    }

    fn send_message(&mut self, client_id: u8, message: String) {
        let session_id = rand::random();
        let fragments = self.deassemble_message(message.as_bytes().to_vec(), session_id);
        for fragment in fragments {
            let packet = Packet {
                pack_type: PacketType::MsgFragment(fragment),
                session_id,
                routing_header: SourceRoutingHeader {
                    hop_index: 1,
                    hops: Vec::new()
                }
            };
            self.send_packet(client_id, packet);
        }
    }

    fn send_flood_request(&mut self) {
        for (client_id, sender) in self.senders() {
            let packet = Packet {
                pack_type: PacketType::FloodRequest(FloodRequest {
                    initiator_id: self.client_id(),
                    flood_id: rand::random(),
                    path_trace: Vec::new(),
                }),
                session_id: rand::random(),
                routing_header: SourceRoutingHeader {
                    hop_index: 1,
                    hops: Vec::new()
                }
            };
            sender.send(packet).unwrap();
        }
    }
}
