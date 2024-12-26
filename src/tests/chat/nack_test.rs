#[cfg(test)]
pub mod nack_test {
    use wg_2024::packet::Nack;
    use wg_2024::{
        network::SourceRoutingHeader,
        packet::{Fragment, Packet, PacketType},
    };

    use crate::client::Client;
    use crate::tests::util;

    /// Test that the client is sending a flood request when reveiving a nack
    #[test]
    fn test_flood_request_sent_on_nack_received() {
        let (mut chat_client, neighbor, _controller_channel_commands, _controller_channel_messages) =
            util::build_client();

        chat_client.sent_packets().insert(0, vec![]);
        chat_client
            .sent_packets()
            .get_mut(&0)
            .unwrap()
            .push(Packet {
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

        let packet = Packet {
            pack_type: PacketType::Nack(Nack {
                nack_type: wg_2024::packet::NackType::ErrorInRouting(2),
                fragment_index: 0,
            }),
            routing_header: SourceRoutingHeader {
                hops: vec![21, 2, 1],
                hop_index: 1,
            },
            session_id: 0,
        };

        chat_client.on_drone_packet_received(Ok(packet));
        let packet_received = neighbor.1.recv().unwrap();

        assert!(matches!(
            packet_received.pack_type,
            PacketType::FloodRequest(_)
        ));
        let packet_received = neighbor.1.recv().unwrap();

        assert!(matches!(
            packet_received.pack_type,
            PacketType::MsgFragment(_)
        ));
    }
}
