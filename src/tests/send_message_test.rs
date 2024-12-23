#[cfg(test)]
pub mod send_message_test {
    use std::collections::HashMap;

    use crossbeam_channel::{unbounded, Receiver, Sender};
    use wg_2024::network::SourceRoutingHeader;
    use wg_2024::packet::{Packet, PacketType};

    use rustafarian_shared::assembler::disassembler::Disassembler;
    use rustafarian_shared::messages::chat_messages::{
        ChatRequest, ChatResponse, ChatResponseWrapper,
    };

    use crate::{chat_client::ChatClient, client::Client};

    #[test]
    fn simple_send_message() {
        let message = "Hello, world".to_string();

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

        chat_client.send_chat_message(21, 3, message.clone());

        let message_req = ChatRequest::SendMessage {
            from: client_id,
            to: 3,
            message,
        };
        let serialized_message = serde_json::to_string(&message_req).unwrap();

        let fragments =
            Disassembler::new().disassemble_message(serialized_message.as_bytes().to_vec(), 0);

        let received_packet = neighbor.1.recv().unwrap();

        let fragment = match received_packet.pack_type {
            PacketType::MsgFragment(fragment) => fragment,
            _ => panic!("Packet type should be MsgFragment"),
        };

        assert_eq!(fragment.data, fragments[0].data);
    }

    #[test]
    fn send_longer_message() {
        let message = "Hello, world".repeat(300).to_string();

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

        chat_client.send_chat_message(21, 3, message.clone());

        let message_req = ChatRequest::SendMessage {
            from: client_id,
            to: 3,
            message,
        };
        let serialized_message = serde_json::to_string(&message_req).unwrap();

        let fragments =
            Disassembler::new().disassemble_message(serialized_message.as_bytes().to_vec(), 0);

        for frag_index in 0..fragments.len() {
            let received_packet = neighbor.1.recv().unwrap();

            let fragment = match received_packet.pack_type {
                PacketType::MsgFragment(fragment) => fragment,
                _ => panic!("Packet type should be MsgFragment"),
            };

            assert_eq!(fragment.data, fragments[frag_index].data);
        }
    }

    #[test]
    fn test_message_sent() {
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

        let message = ChatResponse::MessageSent {};
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
        assert!(true);
    }
}
