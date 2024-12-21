#[cfg(test)]
pub mod nack_test {
    use std::collections::HashMap;

    use crossbeam_channel::{unbounded, Receiver, Sender};
    use rustafarian_shared::assembler::disassembler::Disassembler;
    use rustafarian_shared::messages::chat_messages::ChatResponseWrapper;
    use rustafarian_shared::messages::chat_messages::ChatResponse;
    use wg_2024::packet::Nack;
    use wg_2024::{
        network::SourceRoutingHeader,
        packet::{Fragment, Packet, PacketType},
    };

    use crate::chat_client::ChatClient;
    use crate::client::Client;

    /// Test that the client is sending a flood request when reveiving a nack
    #[test]
    fn test_flood_request_sent_on_nack_received() {
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

        chat_client.sent_packets().insert(0, vec![]);
        chat_client.sent_packets().get_mut(&0).unwrap().push(Packet {
            pack_type: PacketType::MsgFragment(Fragment {
                fragment_index: 0,
                total_n_fragments: 1,
                length: 10,
                data: [0; 128],
            }),
            routing_header: SourceRoutingHeader {
                hops: vec![1, 2, 21],
                hop_index: 1,
            },
            session_id: 0,
        });

        let message = ChatResponse::MessageFrom {
            from: 3,
            message: "Hi".as_bytes().to_vec(),
        };
        let message = ChatResponseWrapper::Chat(message);
        let message_serialized = serde_json::to_string(&message).unwrap();
        let fragments =
            Disassembler::new().disassemble_message(message_serialized.as_bytes().to_vec(), 0);

        let packet = Packet {
            pack_type: PacketType::Nack(Nack {
                nack_type: wg_2024::packet::NackType::ErrorInRouting(2),
                fragment_index: 0
            }),
            routing_header: SourceRoutingHeader {
                hops: vec![21, 2, 1],
                hop_index: 1,
            },
            session_id: 0,
        };

        chat_client.on_drone_packet_received(Ok(packet));
        let packet_received = neighbor.1.recv().unwrap();

        assert!(matches!(packet_received.pack_type, PacketType::FloodRequest(_)));
        let packet_received = neighbor.1.recv().unwrap();

        assert!(matches!(packet_received.pack_type, PacketType::MsgFragment(_)));
    }
}
