#[cfg(test)]
pub mod flooding_test {
    use wg_2024::{
        network::SourceRoutingHeader,
        packet::{Ack, FloodResponse, NodeType, Packet, PacketType},
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

        chat_client.topology().clear();

        chat_client.sent_flood_ids().push(1);
        chat_client.on_flood_response_received(FloodResponse {
            flood_id: 1,
            path_trace: [
                (1, NodeType::Drone),
                (2, NodeType::Drone),
                (21, NodeType::Server),
                (3, NodeType::Client)
            ]
            .to_vec(),
        });

        assert_eq!(
            chat_client.topology().nodes(),
            &vec![1, 2, 21, 3],
            "Topology should contain nodes 1, 2, and 21"
        );

        println!("{:?}", chat_client.topology());

        assert_eq!(
            chat_client.topology().get_node_type(21).unwrap(),
            &"server".to_string()
        );
        assert_eq!(
            chat_client.topology().get_node_type(3).unwrap(),
            &"client".to_string()
        );
        assert_eq!(
            chat_client.topology().get_node_type(1).unwrap(),
            &"drone".to_string()
        );
        assert_eq!(
            chat_client.topology().get_node_type(2).unwrap(),
            &"drone".to_string()
        );

        // assert_eq!(chat_client.topology().edges(), &hash_map![(1, 2), (2, 21)], "Topology should contain edges (1, 2) and (2, 21)");
    }

    #[test]
    fn test_receive_response2() {
        let (
            mut chat_client,
            neighbor,
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
        chat_client.sent_flood_ids().push(0);
        chat_client.packets_to_send().insert(21, Packet {
            pack_type: PacketType::Ack(Ack {
                fragment_index: 0
            }),
            routing_header: SourceRoutingHeader {
                hops: vec![1, 3, 21],
                hop_index: 1,
            },
            session_id: 0,
        });
        chat_client.on_drone_packet_received(Ok(packet));

        assert_eq!(
            chat_client.topology().nodes(),
            &vec![2, 21],
            "Topology should contain nodes 1, 2, and 21"
        );

        assert!(
            chat_client.topology().edges().get(&1).unwrap().contains(&2),
        );

        assert!(
            chat_client.topology().edges().get(&2).unwrap().contains(&21),
        );

        assert!(
            chat_client.topology().edges().get(&2).unwrap().contains(&1),
        );

        assert!(
            chat_client.topology().edges().get(&21).unwrap().contains(&2),
        );

        // First, it sends the server type request, ignore
        // let _packet_received = neighbor.1.recv().unwrap();

        // Now it should send the ack with the new route
        let packet_received = neighbor.1.recv().unwrap();
        println!("{:?}", packet_received);
        assert!(matches!(
            packet_received.pack_type,
            PacketType::Ack(_)
        ));
        assert_eq!(
            packet_received.routing_header.hops,
            vec![1, 2, 21]
        );
    }
}
