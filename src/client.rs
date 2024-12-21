use std::collections::HashMap;

use rustafarian_shared::topology::{compute_route, Topology};

use crossbeam_channel::{select_biased, Receiver, Sender};
use rustafarian_shared::assembler::{assembler::Assembler, disassembler::Disassembler};
use rustafarian_shared::messages::general_messages::{DroneSend, Message, Request, Response};
use wg_2024::packet::{Ack, Fragment, Nack, NackType};
use wg_2024::{
    network::*,
    packet::{FloodRequest, FloodResponse, Packet, PacketType},
};
pub const FRAGMENT_DSIZE: usize = 128;

/// A trait for a client that can send and receive messages
pub trait Client {
    type RequestType: Request;
    type ResponseType: Response;
    type SimControllerMessage: DroneSend; // Message that can be sent to the sim controller
    type SimControllerCommand: DroneSend; // Commands received from the simcontroller

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
    fn sim_controller_receiver(&self) -> &Receiver<Self::SimControllerCommand>;
    /// The channel where the simulation controller can receive messages
    fn sim_controller_sender(&self) -> &Sender<Self::SimControllerMessage>;
    /// Handle a response received from the server
    fn handle_response(&mut self, response: Self::ResponseType, sender_id: NodeId);
    /// Handle a command received from the simulation controller
    fn handle_controller_commands(&mut self, command: Self::SimControllerCommand);
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
            Ok(message) => self.handle_response(message.content, message.source_id),
            Err(str) => panic!("{}", str),
        }
    }

    /// When a FloodResponse is received from a Drone
    /// Behavior: Add the nodes to the topology, and add the edges based on the order of the hops
    /// Then, send ACK
    fn on_flood_response_received(&mut self, flood_response: FloodResponse) {
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

    /// When a fragment is received from a Drone
    /// Behavior: recompose the original message from the fragments. If the message is completed, call on_text_response_arrived
    fn on_fragment_received(&mut self, packet: Packet, fragment: Fragment) {
        let source_id = packet.routing_header.hops[0];
        let fragment_index = fragment.fragment_index;
        // If the message is complete
        if let Some(message) = self.assembler().add_fragment(fragment, packet.session_id) {
            let message_str = String::from_utf8_lossy(&message);
            self.on_text_response_arrived(source_id, packet.session_id, message_str.to_string());
        }
        self.send_ack(fragment_index, source_id);
    }

    /// When a NACK (Negative Acknowledgment) is received
    /// Behavior: send a flood request to update the topology if the problem was in the routing
    /// Then, resend the packet
    fn on_nack_received(&mut self, packet: Packet, nack: Nack) {
        if !matches!(nack.nack_type, NackType::Dropped) {
            self.send_flood_request();
        }
        match self.sent_packets().get(&packet.session_id) {
            Some(lost_packet) => {
                let lost_packet = lost_packet.clone();
                self.send_packet(lost_packet);
            }
            None => {
                eprintln!(
                    "Packet with session_id: {} not found?! Packet list: {:?}",
                    packet.session_id,
                    self.sent_packets()
                );
            }
        }
    }

    fn on_ack_received(&mut self, packet: Packet, ack: Ack) {
        // self.sent_packets().get_mut();
    }

    /// Handle a packet received from a drone based on the type
    fn on_drone_packet_received(&mut self, packet: Result<Packet, crossbeam_channel::RecvError>) {
        if packet.is_err() {
            eprintln!(
                "Client {}: Error receiving packet: {:?}",
                self.client_id(),
                packet.err().unwrap()
            );
            return;
        }
        let packet = packet.unwrap();
        let packet_type = packet.pack_type.clone();
        match packet_type {
            // Handle text fragment
            PacketType::MsgFragment(fragment) => {
                self.on_fragment_received(packet, fragment);
            }
            // Handle flood response
            PacketType::FloodResponse(flood_response) => {
                self.on_flood_response_received(flood_response);
            }
            PacketType::Nack(nack) => {
                self.on_nack_received(packet, nack);
            }
            PacketType::Ack(ack) => {
                self.on_ack_received(packet, ack);
            }
            _ => {
                println!(
                    "Client {} received an unsupported packet type",
                    self.client_id()
                );
            }
        }
    }

    fn handle_sim_controller_packets(
        &mut self,
        packet: Result<Self::SimControllerCommand, crossbeam_channel::RecvError>,
    ) {
        match packet {
            Ok(packet) => self.handle_controller_commands(packet),
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
                    self.on_drone_packet_received(packet);
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
                    hops: compute_route(&self.topology(), client_id, destination_id),
                },
            };
            self.send_packet(packet);
        }
    }

    fn send_ack(&mut self, fragment_index: u64, destination_id: u8) {
        let session_id = rand::random();
        let client_id = self.client_id();
        let packet = Packet {
            pack_type: PacketType::Ack(Ack { fragment_index }),
            session_id,
            routing_header: SourceRoutingHeader {
                hop_index: 1,
                hops: compute_route(&self.topology(), client_id, destination_id),
            },
        };
        self.send_packet(packet);
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
