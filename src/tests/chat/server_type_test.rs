#[cfg(test)]
pub mod server_type_test {
    use rustafarian_shared::assembler::assembler::Assembler;
    use rustafarian_shared::messages::general_messages::{
        ServerType, ServerTypeRequest, ServerTypeResponse,
    };
    use wg_2024::network::SourceRoutingHeader;
    use wg_2024::packet::{Packet, PacketType};

    use rustafarian_shared::messages::chat_messages::ChatResponseWrapper;
    use rustafarian_shared::{
        assembler::disassembler::Disassembler, messages::chat_messages::ChatRequestWrapper,
    };

    use crate::client::Client;
    use crate::tests::util;

    #[test]
    fn test_server_type_request() {
        let (mut chat_client, neighbor, _controller_channel_commands, _controller_channel_messages) =
            util::build_client();

        chat_client.send_server_type_request(21);

        let packet_received = neighbor.1.recv().unwrap();

        let fragment = match packet_received.pack_type {
            PacketType::MsgFragment(fragment) => fragment,
            _ => panic!("Packet type should be ServerTypeRequest"),
        };

        let assembled_message = Assembler::new()
            .add_fragment(fragment, packet_received.session_id)
            .unwrap();

        let parsed_message = match serde_json::from_slice(&assembled_message) {
            Ok(ChatRequestWrapper::ServerType(from)) => from,
            _ => panic!("Message should be ServerTypeRequest"),
        };

        assert!(matches!(parsed_message, ServerTypeRequest::ServerType));
    }

    #[test]
    fn test_server_type_response() {
        let (
            mut chat_client,
            _neighbor,
            _controller_channel_commands,
            _controller_channel_messages,
        ) = util::build_client();

        let server_type_response =
            ChatResponseWrapper::ServerType(ServerTypeResponse::ServerType(ServerType::Chat));

        let serialized_message = serde_json::to_string(&server_type_response).unwrap();

        let fragments =
            Disassembler::new().disassemble_message(serialized_message.as_bytes().to_vec(), 0);

        let packet = Packet {
            pack_type: PacketType::MsgFragment(fragments.get(0).unwrap().clone()),
            routing_header: SourceRoutingHeader {
                hops: vec![21, 2, 1],
                hop_index: 1,
            },
            session_id: 0,
        };

        chat_client.on_drone_packet_received(Ok(packet));

        assert!(chat_client.get_client_list().contains_key(&21));
    }

    #[test]
    fn different_server_type_response() {
        let (
            mut chat_client,
            _neighbor,
            _controller_channel_commands,
            _controller_channel_messages,
        ) = util::build_client();

        let server_type_response =
            ChatResponseWrapper::ServerType(ServerTypeResponse::ServerType(ServerType::Text));

        let serialized_message = serde_json::to_string(&server_type_response).unwrap();

        let fragments =
            Disassembler::new().disassemble_message(serialized_message.as_bytes().to_vec(), 0);

        let packet = Packet {
            pack_type: PacketType::MsgFragment(fragments.get(0).unwrap().clone()),
            routing_header: SourceRoutingHeader {
                hops: vec![21, 2, 1],
                hop_index: 1,
            },
            session_id: 0,
        };

        chat_client.on_drone_packet_received(Ok(packet));

        assert!(!chat_client.get_client_list().contains_key(&21));
    }
}
