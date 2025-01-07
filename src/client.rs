use std::collections::HashMap;

use rustafarian_shared::messages::commander_messages::{
    SimControllerCommand, SimControllerEvent, SimControllerMessage, SimControllerResponseWrapper,
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
    type RequestType: Request; // Represents the type of request the client can send to the server
    type ResponseType: Response; // Represents the type of response the client can receive from the server

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
    /// Packets that need to be sent, as the path couldn't be found. Key: the destination id, Value: the packet
    fn packets_to_send(&mut self) -> &mut HashMap<u8, Packet>;
    /// The list of flood ids that have been sent
    fn sent_flood_ids(&mut self) -> &mut Vec<u64>;
    /// Whether there is a flood request in progress
    fn last_flood_timestamp(&mut self) -> &mut u128;

    /// Deserializes the raw content into the response type
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
        // Deserialize the raw content into the response type, then handle the response
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
            // Add the node to the topology if it doesn't exist
            if !self.topology().nodes().contains(&node.0) {
                self.topology().add_node(node.0);
            }
            // Add the edge between the current node and the previous node in the path trace
            if i > 0 {
                // If the edge already exists, skip
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

            if NodeType::Server == node.1 && self.topology().get_node_type(node.0).is_none() {
                self.topology().set_node_type(node.0, "Server".to_string());
                self.send_server_type_request(node.0);
            }
        }

        // Notify the simulation controller that a flood response has been received
        let _res = self
            .sim_controller_sender()
            .send(SimControllerResponseWrapper::Message(
                SimControllerMessage::FloodResponse(flood_response.flood_id),
            ));

        // Send all the packets that couldn't be sent before
        let packets_to_send = self.packets_to_send().clone();
        self.packets_to_send().clear();
        for packet in packets_to_send {
            // First, update the routing header with the new topology
            let mut new_packet = packet.1.clone();
            let client_id = self.client_id();
            let destination_id = packet.0;
            new_packet.routing_header.hops =
                compute_route(self.topology(), client_id, destination_id);
            self.send_packet(new_packet, destination_id);
        }
    }

    /// When a fragment is received from a Drone
    /// Behavior: recompose the original message from the fragments. If the message is completed, call on_text_response_arrived
    fn on_fragment_received(&mut self, packet: Packet, fragment: Fragment) {
        println!(
            "Client {}: Received fragment {}",
            self.client_id(),
            fragment.fragment_index
        );
        let source_id = packet.routing_header.hops[0];
        let fragment_index = fragment.fragment_index;
        // If the message is complete
        if let Some(message) = self.assembler().add_fragment(fragment, packet.session_id) {
            // Convert the message to a string, then call on_text_response_arrived
            let message_str = String::from_utf8_lossy(&message);
            self.on_text_response_arrived(source_id, packet.session_id, message_str.to_string());
        }
        println!("Source_id: {}", source_id);
        // After receiving a fragment, send an ACK to the source
        self.send_ack(fragment_index, source_id);
    }

    /// When a NACK (Negative Acknowledgment) is received
    /// Behavior: send a flood request to update the topology if the problem was in the routing
    /// Then, resend the packet
    fn on_nack_received(&mut self, packet: Packet, nack: Nack) {
        println!(
            "Client {}: Received NACK ({:?}) for fragment {}",
            self.client_id(),
            nack,
            nack.fragment_index
        );
        // If the NACK is not due to a dropped packet (so the topology was wrong/changed), send a flood request
        if !matches!(nack.nack_type, NackType::Dropped) {
            self.send_flood_request();
        }
        // If the NACK is due to an error in routing (the node crashed), remove the node from the topology
        if let NackType::ErrorInRouting(error_id) = nack.nack_type {
            self.topology().remove_node(error_id);
        }
        // Resend the packet
        match self.sent_packets().get(&packet.session_id) {
            Some(sent_packets) => {
                if (sent_packets.len() as u64) < nack.fragment_index {
                    panic!("Error: NACK fragment index is bigger than the fragment list in the list (f_i: {}, lost_packet list: {:?}", nack.fragment_index, sent_packets);
                }
                let lost_packet = sent_packets
                    .get(nack.fragment_index as usize)
                    .unwrap()
                    .clone();
                let destination_id = lost_packet.routing_header.get_reversed().hops[0];
                self.send_packet(lost_packet, destination_id);
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
        println!(
            "Client {}: Received ACK for fragment {}",
            self.client_id(),
            ack.fragment_index
        );
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
        println!(
            "Client {}: Received flood request: {:?}",
            self.client_id(),
            request
        );
        let sender_id = request.path_trace.last().unwrap().0;
        request.increment(self.client_id(), NodeType::Client);
        let response = Packet::new_flood_request(
            SourceRoutingHeader::empty_route(),
            packet.session_id,
            request,
        );
        // Send the flood request to all neighbors, aside from the sender
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
                let flood_id = flood_response.flood_id;
                self.on_flood_response_received(flood_response);
                if !self.sent_flood_ids().contains(&flood_id) {
                    let mut new_packet = packet.clone();
                    new_packet.routing_header.increase_hop_index();
                    let destination_id = new_packet.routing_header.get_reversed().hops[0];
                    self.send_packet(new_packet, destination_id);
                }
            }
            // Handle NACK (Negative Acknowledgment)
            PacketType::Nack(nack) => {
                self.on_nack_received(packet, nack);
            }
            // Handle ACK (Acknowledgment)
            PacketType::Ack(ack) => {
                self.on_ack_received(packet, ack);
            }
            // Handle flood request
            PacketType::FloodRequest(request) => {
                self.on_flood_request_received(packet, request);
            }
        }
    }

    /// Handle packets received from the simulation controller
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
        // Add the neighbors to the topology
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
        // Send flood request on start, as the topology only contains the neighbors
        self.send_flood_request();
        // Run the client for a certain number of ticks
        while ticks > 0 {
            // Select the first available message from the receiver or the simulation controller receiver
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
        println!("Client {} stopped", self.client_id());
    }

    /// Send a packet to a server
    fn send_packet(&mut self, message: Packet, destination_id: u8) {
        println!(
            "[Client {}] Sending packet: {:?}",
            self.client_id(),
            message.clone()
        );
        let planned_route = message.routing_header.hops.clone();

        // There is no path to the destination
        if planned_route.is_empty() {
            println!(
                "Client {}: No path to destination ({}) for packet: {:?}, current topology: {:?}",
                self.client_id(),
                destination_id,
                message,
                self.topology()
            );
            // Add the packet to the list of packets to send when receiving a flood response
            self.packets_to_send().insert(destination_id, message);
            return;
        }

        // Add the packet to the list of sent packets, in case it needs to be resent (due to nack)
        self.sent_packets()
            .entry(message.session_id)
            .or_default()
            .push(message.clone());
        let packet_type = message.pack_type.clone();
        // The packet is a fragment, I need to receive the ACKs for all the fragments
        if let PacketType::MsgFragment(fragment) = packet_type.clone() {
            self.acked_packets()
                .entry(message.session_id)
                .or_insert(vec![false; fragment.total_n_fragments as usize]);
        }
        let drone_id = message.routing_header.hops[message.routing_header.hop_index];
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
        println!(
            "Client {}: Sending text message to server {}",
            self.client_id(),
            destination_id
        );
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
            self.send_packet(packet, destination_id);
        }
    }

    /// Send an ACK (Acknowledgment) to a server after receiving a fragment
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
        self.send_packet(packet, destination_id);
    }

    /// Send flood request to the neighbors
    fn send_flood_request(&mut self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let timeout = rustafarian_shared::TIMEOUT_BETWEEN_FLOODS_MS as u128;
        // Return if the flood was started less than 500 ms ago
        if *self.last_flood_timestamp() + timeout > now {
            return;
        }
        // Set the last flood timestamp to the current time
        *self.last_flood_timestamp() = now;

        println!("Client {}: Sending flood request", self.client_id());
        let self_id = self.client_id();
        let flood_id = rand::random();
        self.sent_flood_ids().push(flood_id);
        for sender in self.senders() {
            let packet = Packet {
                pack_type: PacketType::FloodRequest(FloodRequest {
                    initiator_id: self.client_id(),
                    flood_id,
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
        // Notify the simulation controller that a flood request has been sent
        self.sim_controller_sender()
            .send(SimControllerResponseWrapper::Event(
                SimControllerEvent::FloodRequestSent,
            ))
            .unwrap();
    }
}
