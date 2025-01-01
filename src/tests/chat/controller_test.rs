#[cfg(test)]
pub mod controller_test {
    use crossbeam_channel::{unbounded, Sender};
    use rustafarian_shared::assembler::assembler::Assembler;
    use rustafarian_shared::messages::chat_messages::{
        ChatRequest, ChatRequestWrapper, ChatResponseWrapper,
    };
    use rustafarian_shared::messages::commander_messages::{
        SimControllerCommand, SimControllerMessage, SimControllerResponseWrapper,
    };
    use rustafarian_shared::messages::general_messages::{ServerType, ServerTypeResponse};
    use wg_2024::packet::{Packet, PacketType};

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

    #[test]
    fn registered_servers_request() {
        let (mut chat_client, _neighbor, _controller_channel_commands, controller_channel_messages) =
            util::build_client();

        chat_client.get_registered_servers().push(21);

        let rs_request = SimControllerCommand::RegisteredServers;

        chat_client.handle_sim_controller_packets(Ok(rs_request));

        let received_packet = controller_channel_messages.1.recv().unwrap();

        let message = match received_packet {
            SimControllerResponseWrapper::Message(message) => message,
            _ => panic!("Packet type should be Message"),
        };

        let rs_msg = match message {
            SimControllerMessage::RegisteredServersResponse(response) => response,
            _ => panic!("Message should be Topology"),
        };

        assert_eq!(rs_msg.clone(), chat_client.get_registered_servers().clone());
    }

    #[test]
    fn known_servers_request() {
        let (mut chat_client, _neighbor, _controller_channel_commands, controller_channel_messages) =
            util::build_client();

        let server_type_response =
            ChatResponseWrapper::ServerType(ServerTypeResponse::ServerType(ServerType::Chat));
        let server_type_response2 =
            ChatResponseWrapper::ServerType(ServerTypeResponse::ServerType(ServerType::Chat));
        let server_type_response3 =
            ChatResponseWrapper::ServerType(ServerTypeResponse::ServerType(ServerType::Chat));

        chat_client.handle_response(server_type_response, 21);
        chat_client.handle_response(server_type_response2, 22);
        chat_client.handle_response(server_type_response3, 23);

        let ks_request = SimControllerCommand::KnownServers;

        chat_client.handle_sim_controller_packets(Ok(ks_request));

        // The first 3 are ServerTypeResponse after receiving the server type
        let _ = controller_channel_messages.1.recv().unwrap();
        let _ = controller_channel_messages.1.recv().unwrap();
        let _ = controller_channel_messages.1.recv().unwrap();

        let received_packet = controller_channel_messages.1.recv().unwrap();

        let message = match received_packet {
            SimControllerResponseWrapper::Message(message) => message,
            _ => panic!(
                "Packet type should be Message, but was {:?}",
                received_packet
            ),
        };

        let rs_msg = match message {
            SimControllerMessage::KnownServers(response) => response,
            _ => panic!("Message should be KnownServer, but was {:?}", message),
        };

        let mut left = rs_msg.keys().cloned().collect::<Vec<u8>>();
        let mut right = chat_client
            .get_available_clients()
            .keys()
            .cloned()
            .collect::<Vec<u8>>();

        left.sort();
        right.sort();

        assert_eq!(left, right);
    }

    #[test]
    fn add_sender_request() {
        let (
            mut chat_client,
            _neighbor,
            _controller_channel_commands,
            _controller_channel_messages,
        ) = util::build_client();

        let new_neighbor: Sender<Packet> = unbounded().0;

        let as_request = SimControllerCommand::AddSender(3, new_neighbor);

        chat_client.handle_sim_controller_packets(Ok(as_request));

        assert!(chat_client.senders().contains_key(&3));

        assert!(chat_client.topology().nodes().contains(&3));
        assert!(chat_client.topology().edges().contains_key(&1));
        assert!(chat_client.topology().edges().contains_key(&3));
        assert!(chat_client.topology().edges().get(&1).unwrap().contains(&3));
    }

    #[test]
    fn remove_sender_request() {
        let (
            mut chat_client,
            _neighbor,
            _controller_channel_commands,
            _controller_channel_messages,
        ) = util::build_client();

        let as_request = SimControllerCommand::RemoveSender(2);

        chat_client.handle_sim_controller_packets(Ok(as_request));

        assert!(!chat_client.senders().contains_key(&2));

        assert!(!chat_client.topology().nodes().contains(&2));
        assert!(chat_client.topology().edges().contains_key(&1));
        assert!(!chat_client.topology().edges().contains_key(&2));
        assert!(!chat_client.topology().edges().get(&1).unwrap().contains(&2));
    }
}
