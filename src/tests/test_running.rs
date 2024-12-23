#[cfg(test)]
pub mod test_running {
    use std::{collections::HashMap, thread};

    use crossbeam_channel::{unbounded, Receiver, Sender};
    use rustafarian_shared::{assembler::disassembler::Disassembler, messages::{
        chat_messages::{ChatResponse, ChatResponseWrapper},
        commander_messages::{SimControllerCommand, SimControllerResponseWrapper},
    }};
    use wg_2024::packet::Packet;

    use crate::{chat_client::ChatClient, client::Client};

    #[test]
    fn test_message_received() {
        let neighbor: (Sender<Packet>, Receiver<Packet>) = unbounded();
        let mut neighbors = HashMap::new();
        neighbors.insert(2 as u8, neighbor.0.clone());
        let channel: (Sender<Packet>, Receiver<Packet>) = unbounded();
        let client_id = 1;

        let controller_channel_commands = unbounded();
        let controller_channel_messages = unbounded();

        let mut chat_client = ChatClient::new(
            client_id,
            neighbors,
            channel.1,
            controller_channel_commands.1.clone(),
            controller_channel_messages.0.clone(),
        );

        chat_client.topology().add_node(2);
        chat_client.topology().add_node(21);
        chat_client.topology().add_edge(2, 21);
        chat_client.topology().add_edge(1, 2);

        let message = ChatResponseWrapper::Chat(ChatResponse::MessageFrom {
            from: 3,
            message: "Hello".as_bytes().to_vec(),
        });

        let mut disassembler = Disassembler::new();
        let fragments = disassembler.disassemble_message(
            serde_json::to_string(&message).unwrap().as_bytes().to_vec(),
            0,
        );

        let fake_packet = Packet {
            pack_type: wg_2024::packet::PacketType::MsgFragment(fragments[0].clone()),
            session_id: rand::random(),
            routing_header: wg_2024::network::SourceRoutingHeader {
                hop_index: 1,
                hops: vec![21, 2, 1],
            },
        };

        thread::spawn(move || {
            chat_client.run();
        });

        channel.0.send(fake_packet).unwrap();

        let sim_received = controller_channel_messages.1.recv().unwrap();
        assert!(matches!(
            sim_received,
            SimControllerResponseWrapper::Message(_)
        ));
    }
}
