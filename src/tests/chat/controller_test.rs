#[cfg(test)]
pub mod controller_test {
    use rustafarian_shared::assembler::assembler::Assembler;
    use rustafarian_shared::messages::chat_messages::{ChatRequest, ChatRequestWrapper};
    use rustafarian_shared::messages::commander_messages::{
        SimControllerCommand, SimControllerMessage, SimControllerResponseWrapper,
    };
    use wg_2024::packet::PacketType;

    use crate::client::Client;
    use crate::tests::util;

    #[test]
    fn flood_request_command() {
        let (mut chat_client, neighbor, _controller_channel_commands, _controller_channel_messages) =
            util::build_client();

        let flood_command = SimControllerCommand::FloodRequest;

        chat_client.handle_controller_commands(flood_command);

        let received_packet = neighbor.1.recv().unwrap();

        assert!(
            matches!(received_packet.pack_type, PacketType::FloodRequest(_)),
            "Packet type should be FloodRequest"
        );
    }

    #[test]
    fn register_command() {
        let (mut chat_client, neighbor, _controller_channel_commands, _controller_channel_messages) =
            util::build_client();

        let register_command = SimControllerCommand::Register(21);

        chat_client.handle_controller_commands(register_command);

        let received_packet = neighbor.1.recv().unwrap();

        let fragment = match received_packet.pack_type {
            PacketType::MsgFragment(fragment) => fragment,
            _ => panic!("Packet type should be MsgFragment"),
        };

        let constructed_message = Assembler::new()
            .add_fragment(fragment, received_packet.session_id)
            .unwrap();

        let parsed_message = serde_json::from_str::<ChatRequestWrapper>(
            std::str::from_utf8(&constructed_message).unwrap(),
        )
        .unwrap();

        assert!(matches!(
            parsed_message,
            ChatRequestWrapper::Chat(ChatRequest::Register(1))
        ));
    }

    #[test]
    fn client_list_command() {
        let (mut chat_client, neighbor, _controller_channel_commands, _controller_channel_messages) =
            util::build_client();

        let client_list_command = SimControllerCommand::ClientList(21);

        chat_client.handle_controller_commands(client_list_command);

        let received_packet = neighbor.1.recv().unwrap();

        let fragment = match received_packet.pack_type {
            PacketType::MsgFragment(fragment) => fragment,
            _ => panic!("Packet type should be MsgFragment"),
        };

        let constructed_message = Assembler::new()
            .add_fragment(fragment, received_packet.session_id)
            .unwrap();

        let parsed_message = serde_json::from_str::<ChatRequestWrapper>(
            std::str::from_utf8(&constructed_message).unwrap(),
        )
        .unwrap();

        assert!(matches!(
            parsed_message,
            ChatRequestWrapper::Chat(ChatRequest::ClientList)
        ));
    }

    #[test]
    fn send_message_command() {
        let (mut chat_client, neighbor, _controller_channel_commands, _controller_channel_messages) =
            util::build_client();

        let message = "Hello, world".to_string();

        let send_message_command = SimControllerCommand::SendMessage(message.clone(), 21, 2);

        chat_client.handle_sim_controller_packets(Ok(send_message_command));

        let received_packet = neighbor.1.recv().unwrap();

        let fragment = match received_packet.pack_type {
            PacketType::MsgFragment(fragment) => fragment,
            _ => panic!("Packet type should be MsgFragment"),
        };

        let constructed_message = Assembler::new()
            .add_fragment(fragment, received_packet.session_id)
            .unwrap();

        let parsed_message = serde_json::from_str::<ChatRequestWrapper>(
            std::str::from_utf8(&constructed_message).unwrap(),
        )
        .unwrap();

        assert!(matches!(
            parsed_message,
            ChatRequestWrapper::Chat(ChatRequest::SendMessage {
                to: 2,
                from: 1,
                message: _
            })
        ));
    }

    #[test]
    fn topology_request() {
        let (mut chat_client, _neighbor, _controller_channel_commands, controller_channel_messages) =
            util::build_client();

        let topology_request = SimControllerCommand::Topology;

        chat_client.handle_sim_controller_packets(Ok(topology_request));

        let received_packet = controller_channel_messages.1.recv().unwrap();

        let message = match received_packet {
            SimControllerResponseWrapper::Message(message) => message,
            _ => panic!("Packet type should be Message"),
        };

        let topology_msg = match message {
            SimControllerMessage::TopologyResponse(topology) => topology,
            _ => panic!("Message should be Topology"),
        };

        assert_eq!(topology_msg.edges(), chat_client.topology().edges());
        assert_eq!(topology_msg.nodes(), chat_client.topology().nodes());
    }
}
