#[cfg(test)]
pub mod flood_req_test {
    use std::collections::HashMap;
    use std::time::Duration;

    use crossbeam_channel::{unbounded, Receiver, Sender};

    use wg_2024::packet::{FloodRequest, NodeType};
    use wg_2024::{
        network::SourceRoutingHeader,
        packet::{Packet, PacketType},
    };

    use crate::chat_client::ChatClient;
    use crate::client::Client;

    /// Test that the client is adding itself to the flood request when receiving one
    #[test]
    fn test_client_added_to_flood_request() {
        let neighbor: (Sender<Packet>, Receiver<Packet>) = unbounded();
        let neighbor2: (Sender<Packet>, Receiver<Packet>) = unbounded();
        let mut neighbors = HashMap::new();
        neighbors.insert(2 as u8, neighbor.0);
        neighbors.insert(3 as u8, neighbor2.0);
        let channel: (Sender<Packet>, Receiver<Packet>) = unbounded();
        let client_id = 1;

        let mut chat_client = ChatClient::new(
            client_id,
            neighbors,
            channel.1,
            unbounded().1,
            unbounded().0,
            false,
        );

        chat_client.topology().add_node(2);
        chat_client.topology().add_node(21);
        chat_client.topology().add_edge(2, 21);
        chat_client.topology().add_edge(1, 2);

        let flood_req = PacketType::FloodRequest(FloodRequest {
            flood_id: 0,
            initiator_id: 21,
            path_trace: vec![(21, NodeType::Server), (2, NodeType::Drone)],
        });

        let packet = Packet {
            pack_type: flood_req,
            routing_header: SourceRoutingHeader {
                hops: vec![],
                hop_index: 1,
            },
            session_id: 0,
        };

        chat_client.on_drone_packet_received(Ok(packet));
        let packet_received = neighbor.1.recv_timeout(Duration::from_millis(50));
        let packet_received2 = neighbor2.1.recv().unwrap();

        println!("{:?}", packet_received);

        assert_eq!(packet_received.is_err(), true);

        // assert!(matches!(packet_received.pack_type, PacketType::FloodRequest(_)));
        assert!(matches!(
            packet_received2.pack_type,
            PacketType::FloodRequest(_)
        ));

        let flood_req = match packet_received2.pack_type {
            PacketType::FloodRequest(flood_req) => flood_req,
            _ => panic!("Expected FloodRequest"),
        };

        assert_eq!(
            flood_req.path_trace,
            vec![
                (21, NodeType::Server),
                (2, NodeType::Drone),
                (1, NodeType::Client)
            ]
        );
    }
}
