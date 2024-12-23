#[cfg(test)]
pub mod server_type_test {
    use std::collections::HashMap;

    use crossbeam_channel::{unbounded, Receiver, Sender};
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

    use crate::{chat_client::ChatClient, client::Client};

    #[test]
    fn test_server_type_request() {
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
}
