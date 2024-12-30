use std::collections::HashMap;

use rustafarian_shared::messages::commander_messages::{
    SimControllerCommand, SimControllerEvent, SimControllerMessage, SimControllerResponseWrapper
};
use rustafarian_shared::topology::{compute_route, Topology};

use crossbeam_channel::{select_biased, Receiver, Sender};
use rustafarian_shared::assembler::{assembler::Assembler, disassembler::Disassembler};
use rustafarian_shared::messages::general_messages::{DroneSend, Message, Request, Response};
use wg_2024::packet::{Ack, Fragment, Nack, NackType, NodeType};
use wg_2024::{
    network::*,
    packet::{FloodRequest, FloodResponse, Packet, PacketType},
};
pub const FRAGMENT_DSIZE: usize = 128;
pub static mut DEBUG: bool = false;

/// A trait for a client that can send and receive messages
pub trait Client: Send {
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
    fn sim_controller_receiver(&self) -> &Receiver<SimControllerCommand>;
    /// The channel where the simulation controller can receive messages
    fn sim_controller_sender(&self) -> &Sender<SimControllerResponseWrapper>;
    /// Handle a response received from the server
    fn handle_response(&mut self, response: Self::ResponseType, sender_id: NodeId);
    /// Handle a command received from the simulation controller
    fn handle_controller_commands(&mut self, command: SimControllerCommand);
    /// Contains all the packets sent by the client, in case they need to be sent again
    fn sent_packets(&mut self) -> &mut HashMap<u64, Vec<Packet>>;
    /// Contains the count of all packets with a certain session_id that have been acked
    fn acked_packets(&mut self) -> &mut HashMap<u64, Vec<bool>>;
    /// Send a `Server Type` request to a server
    fn send_server_type_request(&mut self, server_id: NodeId);
    /// Debug flag to stop the client from resending packets
    fn running(&mut self) -> &mut bool;
    /// Packets that need to be sent, as the path couldn't be found. Key: session_id, Value: Packet
    fn packets_to_send(&mut self) -> &mut HashMap<u64, Packet>;

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
    fn on_flood_response_received(&mut self, flood_response: FloodResponse) {
        println!(
            "Client {} received FloodResponse: {:?}",
            self.client_id(),
            flood_response
        );
        for (i, node) in flood_response.path_trace.iter().enumerate() {
            if !self.topology().nodes().contains(&node.0) {
                self.topology().add_node(node.0);
            }
            if i > 0 {
                if self
                    .topology()
                    .edges()
                    .get(&node.0)
                    .unwrap()
                    .contains(&flood_response.path_trace[i - 1].0)
                {
                    continue;
                }
                self.topology()
                    .add_edge(flood_response.path_trace[i - 1].0, node.0);
                self.topology()
                    .add_edge(node.0, flood_response.path_trace[i - 1].0);
            }
        }

        let _res = self
            .sim_controller_sender()
            .send(SimControllerResponseWrapper::Message(
                SimControllerMessage::FloodResponse(flood_response.flood_id),
            ));

        let packets_to_send = self.packets_to_send().clone();
        for packet in packets_to_send {
            self.send_packet(packet.1);
        }
        self.packets_to_send().clear();
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
        println!("Source_id: {}", source_id);
        self.send_ack(fragment_index, source_id);
    }

    /// When a NACK (Negative Acknowledgment) is received
    /// Behavior: send a flood request to update the topology if the problem was in the routing
    /// Then, resend the packet
    fn on_nack_received(&mut self, packet: Packet, nack: Nack) {
        if !matches!(nack.nack_type, NackType::Dropped) {
            self.send_flood_request();
        }
        if let NackType::ErrorInRouting(error_id) = nack.nack_type {
            self.topology().remove_node(error_id);
        }
        match self.sent_packets().get(&packet.session_id) {
            Some(sent_packets) => {
                if (sent_packets.len() as u64) < nack.fragment_index {
                    panic!("Error: NACK fragment index is bigger than the fragment list in the list (f_i: {}, lost_packet list: {:?}", nack.fragment_index, sent_packets);
                }
                let lost_packet = sent_packets
                    .get(nack.fragment_index as usize)
                    .unwrap()
                    .clone();
                self.send_packet(lost_packet);
            }
            None => {
                panic!(
                    "Packet with session_id: {} not found?! Packet list: {:?}",
                    packet.session_id,
                    self.sent_packets()
                );
            }
        }
    }

    /// When an ACK (Acknowledgment) is received
    /// Behavior: Increase the count of ACKs received for that session ID by 1.
    /// If the count is equal to the number of packets sent with that session ID, remove the session ID from the ACKed packets list
    fn on_ack_received(&mut self, packet: Packet, ack: Ack) {
        // Increase the count of acked packets for this session ID
        self.acked_packets()
            .entry(packet.session_id)
            .or_default()
            .insert(ack.fragment_index as usize, true);
        // Get the current count of acked packets for this session ID
        let acked_packet_count = self
            .acked_packets()
            .get(&packet.session_id)
            .unwrap()
            .iter()
            .filter(|x| **x)
            .count();
        // Get the total number of packets with this session id
        let sent_packet_count = self.sent_packets().get(&packet.session_id).unwrap().len();

        // If all packets have received the acknowledgment
        if acked_packet_count >= sent_packet_count {
            self.sent_packets().remove(&packet.session_id);
            self.acked_packets().remove(&packet.session_id);
        }
    }

    /// On flood request received: add itself to the request, then forward to all neighbors
    fn on_flood_request_received(&mut self, packet: Packet, mut request: FloodRequest) {
        let sender_id = request.path_trace.last().unwrap().0;
        request.increment(self.client_id(), NodeType::Client);
        let response = Packet::new_flood_request(
            SourceRoutingHeader::empty_route(),
            packet.session_id,
            request,
        );
        for (neighbor_id, sender) in self.senders() {
            if neighbor_id != &sender_id {
                sender.send(response.clone()).unwrap();
            }
        }
    }

    /// Handle a packet received from a drone based on the type
    fn on_drone_packet_received(&mut self, packet: Result<Packet, crossbeam_channel::RecvError>) {
        if packet.is_err() {
            panic!(
                "Client {}: Error receiving packet: {:?}",
                self.client_id(),
                packet.err().unwrap()
            );
        }
        let packet = packet.unwrap();
        // Notify the simulation controller that a packet has been received
        let _res = self
            .sim_controller_sender()
            .send(SimControllerResponseWrapper::Event(
                SimControllerEvent::PacketReceived(packet.session_id),
            ));

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
            PacketType::FloodRequest(request) => {
                self.on_flood_request_received(packet, request);
            }
        }
    }

    fn handle_sim_controller_packets(
        &mut self,
        packet: Result<SimControllerCommand, crossbeam_channel::RecvError>,
    ) {
        match packet {
            Ok(packet) => self.handle_controller_commands(packet),
            Err(err) => {
                panic!(
                    "Client {}: Error receiving packet from the simulation controller: {:?}",
                    self.client_id(),
                    err
                );
            }
        };
    }

    /// Run the client, listening for incoming messages
    fn run(&mut self, mut ticks: u64) {
        if self.topology().edges().is_empty() {
            let senders = self.senders().clone();
            let client_id = self.client_id();
            for (sender_id, _channel) in senders {
                self.topology().add_node(sender_id);
                self.topology().add_edge(client_id, sender_id);
            }
        }
        println!("Client {} running", self.client_id());
        *self.running() = true;
        while ticks > 0 {
            select_biased! {
                recv(self.sim_controller_receiver()) -> packet => {
                    self.handle_sim_controller_packets(packet);
                }
                recv(self.receiver()) -> packet => {
                    self.on_drone_packet_received(packet);
                }
            }
            ticks -= 1;
        }
        *self.running() = false;
    }

    /// Send a packet to a server
    fn send_packet(&mut self, message: Packet) {
        println!("Sending packet: {:?}", message.clone());
        let planned_route = message.routing_header.hops.clone();

        // There is no path to the destination
        if planned_route.is_empty() {
            println!(
                "Client {}: No path to destination for packet: {:?}",
                self.client_id(),
                message
            );
            self.packets_to_send().insert(message.session_id, message);
            return;
        }

        self.sent_packets()
            .entry(message.session_id)
            .or_default()
            .push(message.clone());
        let packet_type = message.pack_type.clone();
        if let PacketType::MsgFragment(fragment) = packet_type.clone() {
            self.acked_packets()
                .entry(message.session_id)
                .or_insert(vec![false; fragment.total_n_fragments as usize]);
        }
        let drone_id = message.routing_header.hops[1];
        let session_id = message.session_id;
        match self.senders().get(&drone_id) {
            Some(sender) => {
                sender.send(message).unwrap();

                // Notify the simulation controller that a packet has been sent
                let _res = self
                    .sim_controller_sender()
                    .send(SimControllerResponseWrapper::Event(
                        SimControllerEvent::PacketSent {
                            session_id,
                            packet_type: packet_type.to_string(),
                        },
                    ));
            }
            None => {
                panic!(
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
                    hops: compute_route(self.topology(), client_id, destination_id),
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
                hops: compute_route(self.topology(), client_id, destination_id),
            },
        };
        println!("Sending packet: {:?}", packet.clone());
        self.send_packet(packet);
    }

    /// Send flood request to the neighbors
    fn send_flood_request(&mut self) {
        let self_id = self.client_id();
        for sender in self.senders() {
            let packet = Packet {
                pack_type: PacketType::FloodRequest(FloodRequest {
                    initiator_id: self.client_id(),
                    flood_id: rand::random(),
                    path_trace: vec![(self_id, NodeType::Client)],
                }),
                session_id: rand::random(),
                routing_header: SourceRoutingHeader {
                    hop_index: 1,
                    hops: Vec::new(),
                },
            };
            sender.1.send(packet).unwrap();
        }
        self.sim_controller_sender()
            .send(SimControllerResponseWrapper::Event(
                SimControllerEvent::FloodRequestSent,
            ))
            .unwrap();
    }
}
