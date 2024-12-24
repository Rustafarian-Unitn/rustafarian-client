#[cfg(test)]
pub mod register_test {
    use rustafarian_shared::assembler::assembler::Assembler;
    use rustafarian_shared::messages::chat_messages::ChatRequest;
    use wg_2024::packet::{Fragment, PacketType};

    use crate::{client::Client, tests::util};

    #[test]
    fn simple_register() {
        let (mut chat_client, neighbor, _controller_channel_commands, _controller_channel_messages) =
            util::build_client();

        let client_id = 1;
        let register_request = ChatRequest::Register(client_id);
        let register_serialized = serde_json::to_string(&register_request).unwrap();

        // let mut fragments = disassembler.disassemble_message(register_serialized.as_bytes().to_vec(), 0);

        chat_client.send_message(21, register_serialized.clone());

        let expected_fragment = Fragment {
            fragment_index: 0,
            total_n_fragments: 1,
            length: 14,
            data: [
                123, 34, 82, 101, 103, 105, 115, 116, 101, 114, 34, 58, 49, 125, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ],
        };

        let received_fragment = neighbor.1.recv().unwrap();

        assert_eq!(
            received_fragment.clone().pack_type,
            PacketType::MsgFragment(expected_fragment)
        );

        let mut assembler = Assembler::new();

        let fragment = match received_fragment.pack_type {
            PacketType::MsgFragment(fragment) => fragment,
            _ => panic!("Packet type should be MsgFragment"),
        };

        let reassembled_message = assembler.add_fragment(fragment, received_fragment.session_id);

        assert_eq!(
            reassembled_message.unwrap(),
            register_serialized.as_bytes().to_vec()
        );
    }
}
