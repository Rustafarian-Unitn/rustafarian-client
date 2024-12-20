#[cfg(test)]
pub mod controller_test {
    use std::collections::HashMap;

    use crossbeam_channel::{unbounded, Receiver, Sender};
    use rustafarian_shared::assembler::disassembler;
    use rustafarian_shared::assembler::{assembler::Assembler, disassembler::Disassembler};
    use rustafarian_shared::messages::chat_messages::ChatRequest;
    use rustafarian_shared::messages::commander_messages::{SimControllerChatCommand, SimControllerResponseWrapper};
    use wg_2024::packet::{Fragment, Packet, PacketType};

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

        let flood_command = SimControllerChatCommand::FloodRequest;

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

        let register_command = SimControllerChatCommand::Register(21);

        chat_client.handle_controller_commands(register_command);

        let received_packet = neighbor.1.recv().unwrap();

        let fragment = match received_packet.pack_type {
            PacketType::MsgFragment(fragment) => fragment,
            _ => panic!("Packet type should be MsgFragment"),
        };

        let constructed_message = Assembler::new().add_fragment(fragment, received_packet.session_id).unwrap();

        let parsed_message = serde_json::from_str::<ChatRequest>(std::str::from_utf8(&constructed_message).unwrap()).unwrap();

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

        let client_list_command = SimControllerChatCommand::ClientList(21);

        chat_client.handle_controller_commands(client_list_command);

        let received_packet = neighbor.1.recv().unwrap();

        let fragment = match received_packet.pack_type {
            PacketType::MsgFragment(fragment) => fragment,
            _ => panic!("Packet type should be MsgFragment"),
        };

        let constructed_message = Assembler::new().add_fragment(fragment, received_packet.session_id).unwrap();

        let parsed_message = serde_json::from_str::<ChatRequest>(std::str::from_utf8(&constructed_message).unwrap()).unwrap();

        assert!(matches!(parsed_message, ChatRequest::ClientList));
    }

    #[test]
    fn send_message_command() {}

    #[test]
    fn topology_request() {}
}
