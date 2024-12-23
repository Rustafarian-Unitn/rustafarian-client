#[cfg(test)]
pub mod error_tests {
    use std::collections::HashMap;
    use std::mem;

    use crossbeam_channel::{unbounded, Receiver, Sender};
    use rustafarian_shared::assembler::assembler::Assembler;
    use rustafarian_shared::messages::chat_messages::ChatRequest;
    use rustafarian_shared::messages::commander_messages::{
        SimControllerCommand, SimControllerMessage, SimControllerResponseWrapper,
    };
    use rustafarian_shared::messages::general_messages::Message;
    use wg_2024::packet::{Packet, PacketType};

    use crate::chat_client::{self, ChatClient};
    use crate::client::Client;

    fn build_client() -> ChatClient {
        let neighbor: (Sender<Packet>, Receiver<Packet>) = unbounded();
        let mut neighbors = HashMap::new();
        neighbors.insert(2 as u8, neighbor.0);
        let channel: (Sender<Packet>, Receiver<Packet>) = unbounded();
        let client_id = 1;

        let controller_channel_commands = unbounded();
        let controller_channel_messages = unbounded();

        let chat_client = ChatClient::new(
            client_id,
            neighbors,
            channel.1,
            controller_channel_commands.1,
            controller_channel_messages.0,
        );
        chat_client
    }

    #[test]
    pub fn compose_message_error() {
        let chat_client = build_client();

        let result = chat_client.compose_message(21, 0, "asd".to_string());
        assert!(matches!(result, Err(_)));
    }

    #[test]
    #[should_panic]
    pub fn on_text_response_error() {
        let mut chat_client = build_client();

        chat_client.on_text_response_arrived(21, 0, "asd".to_string());
    }
}
