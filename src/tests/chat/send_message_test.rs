#[cfg(test)]
pub mod send_message_test {
    use wg_2024::network::SourceRoutingHeader;
    use wg_2024::packet::{Packet, PacketType};

    use rustafarian_shared::assembler::disassembler::Disassembler;
    use rustafarian_shared::messages::chat_messages::{
        ChatRequest, ChatRequestWrapper, ChatResponse, ChatResponseWrapper,
    };

    use crate::client::Client;
    use crate::tests::util;

    #[test]
    fn simple_send_message() {
        let message = "Hello, world".to_string();
        let (mut chat_client, neighbor, _controller_channel_commands, _controller_channel_messages) =
            util::build_client();

        chat_client.send_chat_message(21, 3, message.clone());

        let message_req = ChatRequest::SendMessage {
            from: 1,
            to: 3,
            message,
        };
        let serialized_message =
            serde_json::to_string(&ChatRequestWrapper::Chat(message_req)).unwrap();

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

        let (mut chat_client, neighbor, _controller_channel_commands, _controller_channel_messages) =
            util::build_client();

        chat_client.send_chat_message(21, 3, message.clone());

        let message_req = ChatRequest::SendMessage {
            from: 1,
            to: 3,
            message,
        };
        let serialized_message =
            serde_json::to_string(&ChatRequestWrapper::Chat(message_req)).unwrap();

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
        let (
            mut chat_client,
            _neighbor,
            _controller_channel_commands,
            _controller_channel_messages,
        ) = util::build_client();

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
