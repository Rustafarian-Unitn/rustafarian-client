use std::collections::HashMap;

use crate::{assembler::{self, assembler::Assembler, deassembler::Deassembler}, message::{
    DroneSend, Message, Request, Response,
}, topology::Topology};
use crossbeam::select;
use crossbeam_channel::{Receiver, Sender};
use wg_2024::{
    network::*,
    packet::{FloodRequest, FloodResponse, Fragment, Packet, PacketType},
};
pub const FRAGMENT_DSIZE: usize = 128;

/// A trait for a client that can send and receive messages
pub trait Client {
    type RequestType: Request;
    type ResponseType: Response;

    /// Returns the client id
    fn client_id(&self) -> u8;
    /// Returns the drones connected to the client
    fn senders(&self) -> &HashMap<u8, Sender<Packet>>;
    /// The channel where the client can receive messages
    fn receiver(&self) -> &Receiver<Packet>;
    /// The assembler used to reassemble messages
    fn assembler(&mut self) -> &mut Assembler;
    /// The deassembler used to fragment messages
    fn deassembler(&mut self) -> &mut Deassembler;
    /// The topology of the network as the client knows
    fn topology(&mut self) -> &mut Topology;
    /// The channel where the simulation controller can send messages
    fn sim_controller_receiver(&self) -> &Receiver<Message<Self::ResponseType>>;
    /// Handle a response received from the server
    fn handle_response(&mut self, response: Self::ResponseType);

    /// Compose a message to send from a raw string
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

    /// Handle a complete text message re-composed from MsgFragments
    fn on_text_response_arrived(&mut self, source_id: NodeId, session_id: u64, raw_content: String) {
        match self.compose_message(source_id, session_id, raw_content) {
            Ok(message) => {
                let response = self.handle_response(message.content);
                // TODO: send ACK
            }
            Err(str) => panic!("{}", str),
        }
    }

    /// Handle a FloodResponse
    fn on_flood_response(&mut self, flood_response: FloodResponse) {
        println!(
            "Client {} received FloodResponse: {:?}",
            self.client_id(), flood_response
        );
        self.topology().clear();
        for (i, node) in flood_response.path_trace.iter().enumerate() {
            self.topology().add_node(node.0);
            if i > 0 {
                self.topology().add_edge(flood_response.path_trace[i - 1].0, node.0);
                self.topology().add_edge(node.0, flood_response.path_trace[i - 1].0);
            }
        }
    }

    /// Handle a packet received from a drone
    fn handle_drone_packets(&mut self, packet: Result<Packet, crossbeam_channel::RecvError>) {
        match packet {
            Ok(packet) => {
                match packet.pack_type {
                    PacketType::MsgFragment(fragment) => {
                        if let Some(message) = self.assembler().add_fragment(fragment, packet.session_id) {
                            let message_str = String::from_utf8_lossy(&message);
                            self.on_text_response_arrived(0, packet.session_id, message_str.to_string());
                        }
                    }
                    PacketType::FloodResponse(flood_response) => {
                        self.on_flood_response(flood_response);
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

    /// Run the client, listening for incoming messages
    fn run(&mut self) {
        loop {
            select! {
                recv(self.receiver()) -> packet => {
                    self.handle_drone_packets(packet);
                }
            }
        }
    }

    /// Send a packet to a server
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

    /// Send a text message to a server
    fn send_message(&mut self, client_id: u8, message: String) {
        let session_id = rand::random();
        let fragments = self.deassembler().add_message(message.as_bytes().to_vec(), session_id);
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

    /// Send flood request to the neighbors
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
