#[cfg(test)]
pub mod flooding_test {
    use std::collections::HashMap;

    use crossbeam_channel::{unbounded, Receiver, Sender};
    use wg_2024::{
        network::SourceRoutingHeader,
        packet::{FloodRequest, FloodResponse, NodeType, Packet, PacketType},
    };

    use crate::{chat_client::ChatClient, client::Client};

    #[test]
    fn test_sending_request() {
        let neighbor: (Sender<Packet>, Receiver<Packet>) = unbounded();
        let mut neighbors = HashMap::new();
        neighbors.insert(1 as u8, neighbor.0);
        let channel: (Sender<Packet>, Receiver<Packet>) = unbounded();
        let mut chat_client =
            ChatClient::new(1, neighbors, channel.1, unbounded().1, unbounded().0);

        // thread::spawn(move || {
        //     chat_client.run();
        // });
        chat_client.send_flood_request();

        let packet = Packet {
            pack_type: PacketType::FloodRequest(FloodRequest {
                initiator_id: 1,
                flood_id: rand::random(),
                path_trace: Vec::new(),
            }),
            session_id: rand::random(),
            routing_header: SourceRoutingHeader {
                hop_index: 1,
                hops: Vec::new(),
            },
        };

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
        let neighbor: (Sender<Packet>, Receiver<Packet>) = unbounded();
        let mut neighbors = HashMap::new();
        neighbors.insert(1 as u8, neighbor.0);
        let channel: (Sender<Packet>, Receiver<Packet>) = unbounded();

        let mut chat_client =
            ChatClient::new(1, neighbors, channel.1, unbounded().1, unbounded().0);

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
            &vec![1, 2, 21],
            "Topology should contain nodes 1, 2, and 21"
        );

        // assert_eq!(chat_client.topology().edges(), &hash_map![(1, 2), (2, 21)], "Topology should contain edges (1, 2) and (2, 21)");
    }
}
