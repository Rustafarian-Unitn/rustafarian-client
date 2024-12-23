#[cfg(test)]
pub mod flooding_test {
    use wg_2024::{
        network::SourceRoutingHeader,
        packet::{FloodResponse, NodeType, Packet, PacketType},
    };

    use crate::{client::Client, tests::util};

    #[test]
    fn test_sending_request() {
        let (mut chat_client, neighbor, _controller_channel_commands, _controller_channel_messages) =
            util::build_client();

        chat_client.send_flood_request();

        assert!(
            matches!(
                neighbor.1.recv().unwrap().pack_type,
                PacketType::FloodRequest(_)
            ),
            "Packet type should be FloodRequest"
        );
    }

    #[test]
    fn test_receive_response() {
        let (
            mut chat_client,
            _neighbor,
            _controller_channel_commands,
            _controller_channel_messages,
        ) = util::build_client();

        chat_client.on_flood_response_received(FloodResponse {
            flood_id: 1,
            path_trace: [
                (1, NodeType::Drone),
                (2, NodeType::Drone),
                (21, NodeType::Server),
            ]
            .to_vec(),
        });

        assert_eq!(
            chat_client.topology().nodes(),
            &vec![2, 21, 1],
            "Topology should contain nodes 1, 2, and 21"
        );

        // assert_eq!(chat_client.topology().edges(), &hash_map![(1, 2), (2, 21)], "Topology should contain edges (1, 2) and (2, 21)");
    }

    #[test]
    fn test_receive_response2() {
        let (
            mut chat_client,
            _neighbor,
            _controller_channel_commands,
            _controller_channel_messages,
        ) = util::build_client();

        let response = FloodResponse {
            flood_id: 0,
            path_trace: vec![(21, NodeType::Server), (2, NodeType::Drone)],
        };

        let packet = Packet {
            pack_type: PacketType::FloodResponse(response),
            routing_header: SourceRoutingHeader {
                hops: vec![21, 2, 1],
                hop_index: 1,
            },
            session_id: 0,
        };

        chat_client.on_drone_packet_received(Ok(packet));

        assert_eq!(
            chat_client.topology().nodes(),
            &vec![2, 21],
            "Topology should contain nodes 1, 2, and 21"
        );
    }
}
