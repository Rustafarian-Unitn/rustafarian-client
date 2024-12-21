#[cfg(test)]
pub mod send_message_test {
    use std::collections::HashMap;

    use crossbeam_channel::{unbounded, Receiver, Sender};
    use wg_2024::packet::{Packet, PacketType};

    use rustafarian_shared::assembler::disassembler::Disassembler;
    use rustafarian_shared::messages::chat_messages::ChatRequest;

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

        chat_client.send_chat_message(21, message.clone());

        let message_req = ChatRequest::SendMessage {
            from: client_id,
            to: 21,
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

        chat_client.send_chat_message(21, message.clone());

        let message_req = ChatRequest::SendMessage {
            from: client_id,
            to: 21,
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
}
