use std::{collections::HashMap, fmt::Debug};

use crate::topology::{self, Topology};
use crossbeam::select;
use crossbeam_channel::{select_biased, Receiver, Sender};
use rustafarian_shared::assembler::{assembler::Assembler, disassembler::Disassembler};
use rustafarian_shared::messages::general_messages::{DroneSend, Message, Request, Response};
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
    fn deassembler(&mut self) -> &mut Disassembler;
    /// The topology of the network as the client knows
    fn topology(&mut self) -> &mut Topology;
    /// The channel where the simulation controller can send messages
    fn sim_controller_receiver(&self) -> &Receiver<Packet>;
    /// The channel where the simulation controller can receive messages
    fn sim_controller_sender(&self) -> &Sender<Packet>;
    /// Handle a response received from the server
    fn handle_response(&mut self, response: Self::ResponseType);
    /// Contains all the packets sent by the client, in case they need to be sent again
    fn sent_packets(&mut self) -> &mut HashMap<u64, Packet>;

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
    fn on_text_response_arrived(
        &mut self,
        source_id: NodeId,
        session_id: u64,
        raw_content: String,
    ) {
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
            self.client_id(),
            flood_response
        );
        self.topology().clear();
        for (i, node) in flood_response.path_trace.iter().enumerate() {
            self.topology().add_node(node.0);
            if i > 0 {
                self.topology()
                    .add_edge(flood_response.path_trace[i - 1].0, node.0);
                self.topology()
                    .add_edge(node.0, flood_response.path_trace[i - 1].0);
            }
        }
    }

    /// Handle a packet received from a drone
    fn handle_drone_packets(&mut self, packet: Result<Packet, crossbeam_channel::RecvError>) {
        match packet {
            Ok(packet) => {
                match packet.pack_type {
                    // Handle text fragment
                    PacketType::MsgFragment(fragment) => {
                        // If the message is complete
                        if let Some(message) =
                            self.assembler().add_fragment(fragment, packet.session_id)
                        {
                            let message_str = String::from_utf8_lossy(&message);
                            self.on_text_response_arrived(
                                0,
                                packet.session_id,
                                message_str.to_string(),
                            );
                        }
                    }
                    // Handle flood response
                    PacketType::FloodResponse(flood_response) => {
                        self.on_flood_response(flood_response);
                    }
                    _ => {
                        println!(
                            "Client {} received an unsupported packet type",
                            self.client_id()
                        );
                    }
                }
            }
            Err(err) => {
                eprintln!(
                    "Client {}: Error receiving packet: {:?}",
                    self.client_id(),
                    err
                );
            }
        }
    }

    fn handle_sim_controller_packets(
        &mut self,
        packet: Result<Packet, crossbeam_channel::RecvError>,
    ) {
        match packet {
            Ok(packet) => match packet.pack_type {
                _ => {
                    println!(
                        "Client {}: Received a packet type from the simulation controller {:?}",
                        self.client_id(),
                        packet
                    );
                }
            },
            Err(err) => {
                eprintln!(
                    "Client {}: Error receiving packet from the simulation controller: {:?}",
                    self.client_id(),
                    err
                );
            }
        };
    }

    /// Run the client, listening for incoming messages
    fn run(&mut self) {
        loop {
            select_biased! {
                recv(self.sim_controller_receiver()) -> packet => {
                    self.handle_sim_controller_packets(packet);
                }
                recv(self.receiver()) -> packet => {
                    self.handle_drone_packets(packet);
                }
            }
        }
    }

    /// Send a packet to a server
    fn send_packet(&mut self, message: Packet) {
        self.sent_packets()
            .insert(message.session_id, message.clone());
        let drone_id = message.routing_header.hops[1];
        match self.senders().get(&drone_id) {
            Some(sender) => {
                sender.send(message).unwrap();
            }
            None => {
                eprintln!(
                    "Client {}: No sender found for client {}",
                    self.client_id(),
                    drone_id
                );
            }
        }
    }

    /// Send a text message to a server
    fn send_message(&mut self, destination_id: u8, message: String) {
        let session_id = rand::random();
        let fragments = self
            .deassembler()
            .disassemble_message(message.as_bytes().to_vec(), session_id);
        let client_id = self.client_id();
        // Send all the fragments to the server
        for fragment in fragments {
            let packet = Packet {
                pack_type: PacketType::MsgFragment(fragment),
                session_id,
                routing_header: SourceRoutingHeader {
                    hop_index: 1,
                    hops: topology::compute_route(&self.topology(), client_id, destination_id),
                },
            };
            self.send_packet(packet);
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
                    hops: Vec::new(),
                },
            };
            sender.send(packet).unwrap();
        }
    }
}
