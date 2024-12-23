#[cfg(test)]
pub mod ack_test {
    use std::thread;
    use std::time::Duration;

    use rustafarian_shared::assembler::disassembler::Disassembler;
    use rustafarian_shared::messages::chat_messages::ChatResponse;
    use rustafarian_shared::messages::chat_messages::{
        ChatRequest, ChatRequestWrapper, ChatResponseWrapper,
    };
    use wg_2024::packet::Ack;
    use wg_2024::{
        network::SourceRoutingHeader,
        packet::{Packet, PacketType},
    };

    use crate::client::Client;
    use crate::tests::util;

    /// Test that the client is sending the ACK when receiving a fragment
    #[test]
    fn test_ack_sent_on_fragment_received() {
        let (mut chat_client, neighbor, _controller_channel_commands, _controller_channel_messages) =
            util::build_client();

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
        let (
            mut chat_client,
            _neighbor,
            _controller_channel_commands,
            _controller_channel_messages,
        ) = util::build_client();

        let message = ChatRequest::SendMessage {
            from: 1,
            to: 3,
            message: "Hi".to_string(),
        };
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
            pack_type: PacketType::Ack(Ack { fragment_index: 0 }),
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
        let (
            mut chat_client,
            _neighbor,
            _controller_channel_commands,
            _controller_channel_messages,
        ) = util::build_client();

        let message = ChatRequest::SendMessage {
            from: 1,
            to: 3,
            message: "Hi".to_string(),
        };
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
            pack_type: PacketType::Ack(Ack { fragment_index: 1 }),
            routing_header: SourceRoutingHeader {
                hops: vec![21, 2, 1],
                hop_index: 1,
            },
            session_id: 0,
        }));

        assert!(chat_client.sent_packets().contains_key(&0));
        assert_eq!(
            chat_client.acked_packets().get(&0).unwrap(),
            &vec![false, true]
        );

        chat_client.on_drone_packet_received(Ok(Packet {
            pack_type: PacketType::Ack(Ack { fragment_index: 0 }),
            routing_header: SourceRoutingHeader {
                hops: vec![21, 2, 1],
                hop_index: 1,
            },
            session_id: 0,
        }));
        assert!(chat_client.acked_packets().get(&0).is_none());
    }

    /// Test that the fragment is sent again if no ack is received
    #[test]
    fn test_ack_not_received() {
        let (mut chat_client, neighbor, _controller_channel_commands, _controller_channel_messages) =
            util::build_client();
        *chat_client.running() = true;

        let message = ChatRequest::SendMessage {
            from: 1,
            to: 3,
            message: "Hi".to_string(),
        };
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

        assert!(matches!(
            neighbor.1.recv().unwrap().pack_type,
            PacketType::MsgFragment(_)
        ));
        thread::sleep(Duration::from_millis(1000));
        assert!(matches!(
            neighbor.1.recv().unwrap().pack_type,
            PacketType::MsgFragment(_)
        ));
    }
}
