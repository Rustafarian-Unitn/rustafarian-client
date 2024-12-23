#[cfg(test)]
pub mod controller_test {
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

    use crate::chat_client::ChatClient;
    use crate::client::Client;

    #[test]
    fn flood_request_command() {
        let neighbor: (Sender<Packet>, Receiver<Packet>) = unbounded();
        let mut neighbors = HashMap::new();
        neighbors.insert(2 as u8, neighbor.0);
        let channel: (Sender<Packet>, Receiver<Packet>) = unbounded();
        let client_id = 1;

        let controller_channel_commands = unbounded();
        let controller_channel_messages = unbounded();

        let mut chat_client = ChatClient::new(
            client_id,
            neighbors,
            channel.1,
            controller_channel_commands.1,
            controller_channel_messages.0,
        );

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
        let neighbor: (Sender<Packet>, Receiver<Packet>) = unbounded();
        let mut neighbors = HashMap::new();
        neighbors.insert(2 as u8, neighbor.0);
        let channel: (Sender<Packet>, Receiver<Packet>) = unbounded();
        let client_id = 1;

        let controller_channel_commands = unbounded();
        let controller_channel_messages = unbounded();

        let mut chat_client = ChatClient::new(
            client_id,
            neighbors,
            channel.1,
            controller_channel_commands.1,
            controller_channel_messages.0,
        );

        chat_client.topology().add_node(2);
        chat_client.topology().add_node(21);
        chat_client.topology().add_edge(2, 21);
        chat_client.topology().add_edge(1, 2);

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

        let parsed_message =
            serde_json::from_str::<ChatRequest>(std::str::from_utf8(&constructed_message).unwrap())
                .unwrap();

        assert!(matches!(parsed_message, ChatRequest::Register(1)));
    }

    #[test]
    fn client_list_command() {
        let neighbor: (Sender<Packet>, Receiver<Packet>) = unbounded();
        let mut neighbors = HashMap::new();
        neighbors.insert(2 as u8, neighbor.0);
        let channel: (Sender<Packet>, Receiver<Packet>) = unbounded();
        let client_id = 1;

        let controller_channel_commands = unbounded();
        let controller_channel_messages = unbounded();

        let mut chat_client = ChatClient::new(
            client_id,
            neighbors,
            channel.1,
            controller_channel_commands.1,
            controller_channel_messages.0,
        );

        chat_client.topology().add_node(2);
        chat_client.topology().add_node(21);
        chat_client.topology().add_edge(2, 21);
        chat_client.topology().add_edge(1, 2);

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

        let parsed_message =
            serde_json::from_str::<ChatRequest>(std::str::from_utf8(&constructed_message).unwrap())
                .unwrap();

        assert!(matches!(parsed_message, ChatRequest::ClientList));
    }

    #[test]
    fn send_message_command() {
        let neighbor: (Sender<Packet>, Receiver<Packet>) = unbounded();
        let mut neighbors = HashMap::new();
        neighbors.insert(2 as u8, neighbor.0);
        let channel: (Sender<Packet>, Receiver<Packet>) = unbounded();
        let client_id = 1;

        let controller_channel_commands = unbounded();
        let controller_channel_messages = unbounded();

        let mut chat_client = ChatClient::new(
            client_id,
            neighbors,
            channel.1,
            controller_channel_commands.1,
            controller_channel_messages.0,
        );

        chat_client.topology().add_node(2);
        chat_client.topology().add_node(21);
        chat_client.topology().add_edge(2, 21);
        chat_client.topology().add_edge(1, 2);

        let message = "Hello, world".to_string();

        let send_message_command = SimControllerCommand::SendMessage(message.clone(), 21, 2);

        chat_client.handle_controller_commands(send_message_command);

        let received_packet = neighbor.1.recv().unwrap();

        let fragment = match received_packet.pack_type {
            PacketType::MsgFragment(fragment) => fragment,
            _ => panic!("Packet type should be MsgFragment"),
        };

        let constructed_message = Assembler::new()
            .add_fragment(fragment, received_packet.session_id)
            .unwrap();

        let parsed_message =
            serde_json::from_str::<ChatRequest>(std::str::from_utf8(&constructed_message).unwrap())
                .unwrap();

        assert!(matches!(
            parsed_message,
            ChatRequest::SendMessage {
                to: 2,
                from: 1,
                message
            }
        ));
    }

    #[test]
    fn topology_request() {
        let neighbor: (Sender<Packet>, Receiver<Packet>) = unbounded();
        let mut neighbors = HashMap::new();
        neighbors.insert(2 as u8, neighbor.0);
        let channel: (Sender<Packet>, Receiver<Packet>) = unbounded();
        let client_id = 1;

        let controller_channel_commands = unbounded();
        let controller_channel_messages = unbounded();

        let mut chat_client = ChatClient::new(
            client_id,
            neighbors,
            channel.1,
            controller_channel_commands.1,
            controller_channel_messages.0,
        );

        chat_client.topology().add_node(2);
        chat_client.topology().add_node(21);
        chat_client.topology().add_edge(2, 21);
        chat_client.topology().add_edge(1, 2);

        let topology_request = SimControllerCommand::Topology;

        chat_client.handle_controller_commands(topology_request);

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
