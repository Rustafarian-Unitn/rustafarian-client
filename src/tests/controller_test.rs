#[cfg(test)]
pub mod controller_test {
    use std::collections::HashMap;

    use crossbeam_channel::{unbounded, Receiver, Sender};
    use rustafarian_shared::messages::commander_messages::SimControllerChatCommand;
    use wg_2024::packet::{Fragment, Packet, PacketType};
    use rustafarian_shared::assembler::{assembler::Assembler, disassembler::Disassembler};
    use rustafarian_shared::messages::chat_messages::ChatRequest;

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
            matches!(
                received_packet.pack_type,
                PacketType::FloodRequest(_)
            ),
            "Packet type should be FloodRequest"
        );
    }
}