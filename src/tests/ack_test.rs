#[cfg(test)]
pub mod ack_test {
    use std::collections::HashMap;

    use crossbeam_channel::{unbounded, Receiver, Sender};
    use rustafarian_shared::assembler::disassembler::Disassembler;
    use rustafarian_shared::messages::chat_messages::{ChatRequest, ChatRequestWrapper, ChatResponseWrapper};
    use rustafarian_shared::{
        assembler::assembler::Assembler, messages::chat_messages::ChatResponse,
    };
    use wg_2024::packet::Ack;
    use wg_2024::{
        network::SourceRoutingHeader,
        packet::{Fragment, Packet, PacketType},
    };

    use crate::{chat_client::ChatClient, client::Client};

    /// Test that the client is sending the ACK when receiving a fragment
    #[test]
    fn test_ack_sent_on_fragment_received() {
        let neighbor: (Sender<Packet>, Receiver<Packet>) = unbounded();
        let mut neighbors = HashMap::new();
        neighbors.insert(2 as u8, neighbor.0);
        let channel: (Sender<Packet>, Receiver<Packet>) = unbounded();
        let client_id = 1;

        let mut chat_client = ChatClient::new(
            client_id,
            neighbors,
            channel.1,
            unbounded().1,
            unbounded().0,
        );

        chat_client.topology().add_node(2);
        chat_client.topology().add_node(21);
        chat_client.topology().add_edge(2, 21);
        chat_client.topology().add_edge(1, 2);

        let message = ChatResponse::MessageFrom {
            from: 3,
            message: "Hi".as_bytes().to_vec(),
        };
        let message = ChatResponseWrapper::Chat(message);
        let message_serialized = serde_json::to_string(&message).unwrap();
        let fragments =
            Disassembler::new().disassemble_message(message_serialized.as_bytes().to_vec(), 0);

        let packet = Packet {
            pack_type: PacketType::MsgFragment(fragments.get(0).unwrap().clone()),
            routing_header: SourceRoutingHeader {
                hops: vec![21, 2, 1],
                hop_index: 1,
            },
            session_id: 0,
        };

        chat_client.on_drone_packet_received(Ok(packet));
        let packet_received = neighbor.1.recv().unwrap();

        assert!(matches!(packet_received.pack_type, PacketType::Ack(_)));
    }

    /// Tests that the client adds the packet sent to the list, and removes it when receiving the ACK from the server
    #[test]
    fn test_ack_sent_packet_added_to_list() {
        let neighbor: (Sender<Packet>, Receiver<Packet>) = unbounded();
        let mut neighbors = HashMap::new();
        neighbors.insert(2 as u8, neighbor.0);
        let channel: (Sender<Packet>, Receiver<Packet>) = unbounded();
        let client_id = 1;

        let mut chat_client = ChatClient::new(
            client_id,
            neighbors,
            channel.1,
            unbounded().1,
            unbounded().0,
        );

        chat_client.topology().add_node(2);
        chat_client.topology().add_node(21);
        chat_client.topology().add_edge(2, 21);
        chat_client.topology().add_edge(1, 2);

        let message = ChatRequest::SendMessage { from: 1, to: 3, message: "Hi".to_string() };
        let message = ChatRequestWrapper::Chat(message);
        let message_serialized = serde_json::to_string(&message).unwrap();
        let fragments =
            Disassembler::new().disassemble_message(message_serialized.as_bytes().to_vec(), 0);

        let packet = Packet {
            pack_type: PacketType::MsgFragment(fragments.get(0).unwrap().clone()),
            routing_header: SourceRoutingHeader {
                hops: vec![1, 2, 21],
                hop_index: 1,
            },
            session_id: 0,
        };

        chat_client.send_packet(packet.clone());
        assert!(chat_client.sent_packets().contains_key(&0));

        chat_client.on_drone_packet_received(Ok(Packet {
            pack_type: PacketType::Ack(Ack {
                fragment_index: 0,
            }),
            routing_header: SourceRoutingHeader {
                hops: vec![21, 2, 1],
                hop_index: 1,
            },
            session_id: 0,
        }));

        assert!(!chat_client.sent_packets().contains_key(&0));
    }

    /// Test that if multiple fragments with the same session_id
    /// are sent, and an ACK is received, the count is updated,
    /// but the fragments are not removed
    #[test]
    fn test_ack_sent_2() {
        let neighbor: (Sender<Packet>, Receiver<Packet>) = unbounded();
        let mut neighbors = HashMap::new();
        neighbors.insert(2 as u8, neighbor.0);
        let channel: (Sender<Packet>, Receiver<Packet>) = unbounded();
        let client_id = 1;

        let mut chat_client = ChatClient::new(
            client_id,
            neighbors,
            channel.1,
            unbounded().1,
            unbounded().0,
        );

        chat_client.topology().add_node(2);
        chat_client.topology().add_node(21);
        chat_client.topology().add_edge(2, 21);
        chat_client.topology().add_edge(1, 2);

        let message = ChatRequest::SendMessage { from: 1, to: 3, message: "Hi".to_string() };
        let message = ChatRequestWrapper::Chat(message);
        let message_serialized = serde_json::to_string(&message).unwrap();
        let fragments =
            Disassembler::new().disassemble_message(message_serialized.as_bytes().to_vec(), 0);

        let packet = Packet {
            pack_type: PacketType::MsgFragment(fragments.get(0).unwrap().clone()),
            routing_header: SourceRoutingHeader {
                hops: vec![1, 2, 21],
                hop_index: 1,
            },
            session_id: 0,
        };

        chat_client.send_packet(packet.clone());
        chat_client.send_packet(packet.clone());
        assert!(chat_client.sent_packets().contains_key(&0));

        chat_client.on_drone_packet_received(Ok(Packet {
            pack_type: PacketType::Ack(Ack {
                fragment_index: 1,
            }),
            routing_header: SourceRoutingHeader {
                hops: vec![21, 2, 1],
                hop_index: 1,
            },
            session_id: 0,
        }));

        assert!(chat_client.sent_packets().contains_key(&0));
        assert_eq!(chat_client.acked_packets().get(&0).unwrap(), &vec![false, true]);

        
        chat_client.on_drone_packet_received(Ok(Packet {
            pack_type: PacketType::Ack(Ack {
                fragment_index: 0,
            }),
            routing_header: SourceRoutingHeader {
                hops: vec![21, 2, 1],
                hop_index: 1,
            },
            session_id: 0,
        }));
        assert!(chat_client.acked_packets().get(&0).is_none());
    }
}
