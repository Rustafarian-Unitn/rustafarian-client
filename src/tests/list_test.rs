#[cfg(test)]
pub mod client_list_test {
    use rustafarian_shared::assembler::disassembler::Disassembler;
    use rustafarian_shared::messages::chat_messages::{ChatResponse, ChatResponseWrapper};
    use wg_2024::packet::{Packet, PacketType};

    use crate::tests::util;
    use crate::client::Client;

    #[test]
    fn test_receive_client_list() {
        let (mut chat_client, _neighbor, _controller_channel_commands, _controller_channel_messages) =
            util::build_client();

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

        chat_client.on_drone_packet_received(Ok(fake_packet));

        println!("Client list aaaa: {:?}", chat_client.get_client_list());

        assert_eq!(
            chat_client.get_client_list().get(&21).unwrap(),
            &vec![11, 12]
        );
    }
}
