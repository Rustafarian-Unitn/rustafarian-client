#[cfg(test)]
pub mod client_list_test {
    use std::collections::HashMap;

    use crossbeam_channel::{unbounded, Receiver, Sender};
    use rustafarian_shared::assembler::disassembler::Disassembler;
    use rustafarian_shared::messages::chat_messages::{ChatResponse, ChatResponseWrapper};
    use wg_2024::packet::{Packet, PacketType};

    use crate::{chat_client::ChatClient, client::Client};

    #[test]
    fn test_receive_client_list() {
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

        let client_list_response =
            ChatResponseWrapper::Chat(ChatResponse::ClientList([11, 12].to_vec()));

        let mut disassembler = Disassembler::new();
        let fragments = disassembler.disassemble_message(
            serde_json::to_string(&client_list_response)
                .unwrap()
                .as_bytes()
                .to_vec(),
            0,
        );
        let fake_packet = Packet {
            pack_type: PacketType::MsgFragment(fragments[0].clone()),
            session_id: rand::random(),
            routing_header: wg_2024::network::SourceRoutingHeader {
                hop_index: 1,
                hops: vec![21, 2, 1],
            },
        };

        chat_client.topology().add_node(2);
        chat_client.topology().add_node(21);
        chat_client.topology().add_edge(2, 21);
        chat_client.topology().add_edge(1, 2);

        chat_client.on_drone_packet_received(Ok(fake_packet));

        println!("Client list aaaa: {:?}", chat_client.get_client_list());

        assert_eq!(
            chat_client.get_client_list().get(&21).unwrap(),
            &vec![11, 12]
        );
    }
}
