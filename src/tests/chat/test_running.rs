#[cfg(test)]
pub mod test_running {
    use std::{collections::HashMap, thread};

    use crossbeam_channel::{unbounded, Receiver, Sender};
    use rustafarian_shared::{
        assembler::{assembler::Assembler, disassembler::Disassembler},
        messages::{
            chat_messages::{ChatRequest, ChatRequestWrapper, ChatResponse, ChatResponseWrapper},
            commander_messages::{SimControllerCommand, SimControllerResponseWrapper},
        },
    };
    use wg_2024::packet::{Packet, PacketType};

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
            controller_channel_commands.1,
            controller_channel_messages.0,
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
            chat_client.run(10);
        });

        let _ = controller_channel_messages.1.recv(); // FloodRequestEvent

        channel.0.send(fake_packet).unwrap();
        let _pack_received_event = controller_channel_messages.1.recv().unwrap();
        let sim_received = controller_channel_messages.1.recv().unwrap();
        assert!(matches!(
            sim_received,
            SimControllerResponseWrapper::Message(_)
        ));
    }

    #[test]
    fn test_sim_controller_command() {
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

        let sim_command = SimControllerCommand::SendMessage("Hello".to_string(), 21, 3);

        thread::spawn(move || {
            chat_client.run(1);
        });
        let _ = controller_channel_messages.1.recv(); // FloodRequestEvent
        let _ = neighbor.1.recv(); // FloodRequest

        controller_channel_commands.0.send(sim_command).unwrap();

        let packet_received = neighbor.1.recv().unwrap();
        assert!(matches!(
            packet_received.pack_type,
            PacketType::MsgFragment(_)
        ));

        let fragment = match packet_received.pack_type {
            PacketType::MsgFragment(fragment) => fragment,
            _ => panic!("Packet type should be MsgFragment"),
        };

        let assembled_message = Assembler::new()
            .add_fragment(fragment, packet_received.session_id)
            .unwrap();

        let parsed_message = match serde_json::from_slice::<ChatRequestWrapper>(&assembled_message)
        {
            Ok(wrapper) => match wrapper {
                ChatRequestWrapper::Chat(request) => match request {
                    ChatRequest::SendMessage { from, to, message } => (message, from, to),
                    _ => panic!("Message should be SendMessage"),
                },
                _ => panic!("Message should be SendMessage"),
            },
            _ => panic!("Message should be MessageTo"),
        };

        assert_eq!(parsed_message.0, "Hello".to_string());
        assert_eq!(parsed_message.1, 1);
        assert_eq!(parsed_message.2, 3);
    }

    #[test]
    fn test_run_topology() {
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

        thread::spawn(move || {
            chat_client.run(1);
            assert!(chat_client.topology().edges().contains_key(&2));
        });
    }
}
