#[cfg(test)]
pub mod register_test {
    use rustafarian_shared::messages::chat_messages::{
        ChatRequest, ChatResponse, ChatResponseWrapper,
    };
    use rustafarian_shared::messages::general_messages::DroneSend;
    use rustafarian_shared::{
        assembler::assembler::Assembler, messages::chat_messages::ChatRequestWrapper,
    };
    use wg_2024::packet::PacketType;

    use crate::client::Client;
    use crate::tests::util;

    #[test]
    fn simple_register() {
        let (mut chat_client, neighbor, _controller_channel_commands, _controller_channel_messages) =
            util::build_client();

        chat_client.register(21);

        let received_fragment = neighbor.1.recv().unwrap();

        assert!(matches!(
            received_fragment.clone().pack_type,
            PacketType::MsgFragment(_)
        ));

        let mut assembler = Assembler::new();

        let fragment = match received_fragment.pack_type {
            PacketType::MsgFragment(fragment) => fragment,
            _ => panic!("Packet type should be MsgFragment"),
        };

        let reassembled_message = assembler.add_fragment(fragment, received_fragment.session_id);
        let binding = reassembled_message.unwrap();
        let reassembled_json = String::from_utf8_lossy(&binding);

        let registered = ChatRequestWrapper::Chat(ChatRequest::Register(1));
        let registered_json = registered.stringify();

        assert_eq!(reassembled_json, registered_json);

        let register_response = ChatResponseWrapper::Chat(ChatResponse::ClientRegistered);

        chat_client.handle_response(register_response, 21);

        assert!(chat_client.get_registered_servers().contains(&21));
    }
}
