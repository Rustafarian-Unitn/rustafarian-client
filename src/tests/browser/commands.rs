#[cfg(test)]
pub mod request_file_list {
    use crossbeam_channel::{unbounded, Sender};
    use rustafarian_shared::{
        assembler::disassembler::Disassembler,
        messages::{
            browser_messages::{BrowserRequest, BrowserRequestWrapper},
            commander_messages::{
                SimControllerCommand, SimControllerMessage, SimControllerResponseWrapper,
            },
            general_messages::DroneSend,
        },
    };
    use wg_2024::{
        network::SourceRoutingHeader,
        packet::{Packet, PacketType},
    };

    use crate::{
        client::Client,
        tests::util::{self, build_browser},
    };

    #[test]
    fn test_command_file_list() {
        let (mut browser_client, neighbor, _, _) = build_browser();

        browser_client.handle_sim_controller_packets(Ok(SimControllerCommand::RequestFileList(21)));

        let file_request = BrowserRequestWrapper::Chat(BrowserRequest::FileList);

        let file_request_json = file_request.stringify();

        let disassembled =
            Disassembler::new().disassemble_message(file_request_json.as_bytes().to_vec(), 0);

        let received_packet = neighbor.1.recv().unwrap();

        let expected_packet = Packet {
            routing_header: SourceRoutingHeader::new(vec![1, 2, 21], 1),
            session_id: received_packet.session_id,
            pack_type: PacketType::MsgFragment(disassembled.get(0).unwrap().clone()),
        };
        assert_eq!(expected_packet, received_packet);
    }

    #[test]
    fn test_command_request_text_file() {
        let (mut browser_client, neighbor, _, _) = build_browser();

        browser_client
            .handle_sim_controller_packets(Ok(SimControllerCommand::RequestTextFile(1, 21)));

        let file_request = BrowserRequestWrapper::Chat(BrowserRequest::TextFileRequest(1));

        let file_request_json = file_request.stringify();

        let disassembled =
            Disassembler::new().disassemble_message(file_request_json.as_bytes().to_vec(), 0);

        let received_packet = neighbor.1.recv().unwrap();

        let expected_packet = Packet {
            routing_header: SourceRoutingHeader::new(vec![1, 2, 21], 1),
            session_id: received_packet.session_id,
            pack_type: PacketType::MsgFragment(disassembled.get(0).unwrap().clone()),
        };

        assert_eq!(expected_packet, received_packet);
    }

    #[test]
    fn test_command_request_media_file() {
        let (mut browser_client, neighbor, _, _) = build_browser();

        browser_client
            .handle_sim_controller_packets(Ok(SimControllerCommand::RequestMediaFile(1, 21)));

        let file_request = BrowserRequestWrapper::Chat(BrowserRequest::MediaFileRequest(1));

        let file_request_json = file_request.stringify();

        let disassembled =
            Disassembler::new().disassemble_message(file_request_json.as_bytes().to_vec(), 0);

        let received_packet = neighbor.1.recv().unwrap();

        let expected_packet = Packet {
            routing_header: SourceRoutingHeader::new(vec![1, 2, 21], 1),
            session_id: received_packet.session_id,
            pack_type: PacketType::MsgFragment(disassembled.get(0).unwrap().clone()),
        };

        assert_eq!(expected_packet, received_packet);
    }

    #[test]
    fn test_sending_request() {
        let (
            mut browser_client,
            neighbor,
            _controller_channel_commands,
            _controller_channel_messages,
        ) = util::build_browser();

        browser_client.handle_controller_commands(SimControllerCommand::FloodRequest);

        assert!(
            matches!(
                neighbor.1.recv().unwrap().pack_type,
                PacketType::FloodRequest(_)
            ),
            "Packet type should be FloodRequest"
        );
    }

    #[test]
    fn topology_request() {
        let (
            mut browser_client,
            _neighbor,
            _controller_channel_commands,
            controller_channel_messages,
        ) = util::build_browser();

        let topology_request = SimControllerCommand::Topology;

        browser_client.handle_sim_controller_packets(Ok(topology_request));

        let received_packet = controller_channel_messages.1.recv().unwrap();

        let message = match received_packet {
            SimControllerResponseWrapper::Message(message) => message,
            _ => panic!("Packet type should be Message"),
        };

        let topology_msg = match message {
            SimControllerMessage::TopologyResponse(topology) => topology,
            _ => panic!("Message should be Topology"),
        };

        assert_eq!(topology_msg.edges(), browser_client.topology().edges());
        assert_eq!(topology_msg.nodes(), browser_client.topology().nodes());
    }

    #[test]
    fn send_unhandled_command() {
        let (
            mut browser_client,
            _neighbor,
            _controller_channel_commands,
            _controller_channel_messages,
        ) = util::build_browser();

        let unhandled_request = SimControllerCommand::ClientList(21);

        browser_client.handle_sim_controller_packets(Ok(unhandled_request));
    }

    #[test]
    fn add_sender_request() {
        let (
            mut chat_client,
            _neighbor,
            _controller_channel_commands,
            _controller_channel_messages,
        ) = util::build_browser();

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
        ) = util::build_browser();

        let as_request = SimControllerCommand::RemoveSender(2);

        chat_client.handle_sim_controller_packets(Ok(as_request));

        assert!(!chat_client.senders().contains_key(&2));

        assert!(!chat_client.topology().nodes().contains(&2));
        assert!(chat_client.topology().edges().contains_key(&1));
        assert!(!chat_client.topology().edges().contains_key(&2));
        assert!(!chat_client.topology().edges().get(&1).unwrap().contains(&2));
    }
}
