#[cfg(test)]
pub mod error_tests {

    use rustafarian_shared::messages::chat_messages::{ChatResponse, ChatResponseWrapper};

    use crate::client::Client;
    use crate::tests::util;

    #[test]
    pub fn compose_message_error() {
        let (chat_client, _neighbor, _controller_channel_commands, _controller_channel_messages) =
            util::build_client();

        let result = chat_client.compose_message(21, 0, "asd".to_string());
        assert!(matches!(result, Err(_)));

        let message = ChatResponseWrapper::Chat(ChatResponse::MessageSent);
        let message_json = serde_json::to_string(&message).unwrap();
        let result = chat_client.compose_message(21, 0, message_json);
        assert!(matches!(result, Ok(_)));
    }

    #[test]
    #[should_panic]
    pub fn on_text_response_error() {
        let (
            mut chat_client,
            _neighbor,
            _controller_channel_commands,
            _controller_channel_messages,
        ) = util::build_client();

        chat_client.on_text_response_arrived(21, 0, "asd".to_string());
    }
}
