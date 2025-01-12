#[cfg(test)]
pub mod command_tests {
    use crossbeam_channel::{unbounded, Sender};
    use rustafarian_shared::{
        assembler::{assembler::Assembler, disassembler::Disassembler},
        messages::{
            browser_messages::{BrowserRequest, BrowserRequestWrapper},
            commander_messages::{
                SimControllerCommand, SimControllerMessage, SimControllerResponseWrapper,
            },
            general_messages::{DroneSend, ServerTypeRequest},
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
            mut browser_client,
            _neighbor,
            _controller_channel_commands,
            _controller_channel_messages,
        ) = util::build_browser();

        let new_neighbor: Sender<Packet> = unbounded().0;

        let as_request = SimControllerCommand::AddSender(3, new_neighbor);

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();

        *browser_client.last_flood_timestamp() = now;

        browser_client.handle_sim_controller_packets(Ok(as_request));

        assert!(browser_client.senders().contains_key(&3));

        assert!(browser_client.topology().nodes().contains(&3));
        assert!(browser_client.topology().edges().contains_key(&1));
        assert!(browser_client.topology().edges().contains_key(&3));
        assert!(browser_client
            .topology()
            .edges()
            .get(&1)
            .unwrap()
            .contains(&3));
    }

    #[test]
    fn remove_sender_request() {
        let (
            mut browser_client,
            _neighbor,
            _controller_channel_commands,
            _controller_channel_messages,
        ) = util::build_browser();

        let as_request = SimControllerCommand::RemoveSender(2);

        browser_client.handle_sim_controller_packets(Ok(as_request));

        assert!(!browser_client.senders().contains_key(&2));

        assert!(browser_client.topology().nodes().contains(&2));
        assert!(browser_client.topology().edges().contains_key(&1));
        assert!(browser_client.topology().edges().contains_key(&2));
        assert!(!browser_client
            .topology()
            .edges()
            .get(&1)
            .unwrap()
            .contains(&2));
        assert!(!browser_client
            .topology()
            .edges()
            .get(&2)
            .unwrap()
            .contains(&1));
    }

    #[test]
    fn known_servers_type_req() {
        let (
            mut browser_client,
            neighbor,
            _controller_channel_commands,
            _controller_channel_messages,
        ) = util::build_browser();

        browser_client
            .topology()
            .set_node_type(21, "server".to_string());

        let ks_request = SimControllerCommand::KnownServers;

        browser_client.handle_sim_controller_packets(Ok(ks_request));

        let received_server_type = neighbor.1.recv().unwrap();

        let destination = received_server_type.routing_header.destination();
        assert!(destination.is_some());
        assert!(destination.unwrap() == 21);

        let fragment = match received_server_type.pack_type {
            PacketType::MsgFragment(fragment) => fragment,
            _ => panic!("Unexpected packet type"),
        };

        let request = Assembler::new().add_fragment(fragment, 0);
        assert!(request.is_some());
        let binding = request.unwrap();
        let request = serde_json::from_slice::<BrowserRequestWrapper>(&binding).unwrap();

        assert!(matches!(
            request,
            BrowserRequestWrapper::ServerType(ServerTypeRequest::ServerType)
        ));
    }

    #[test]
    fn request_server_type() {
        let (
            mut browser_client,
            neighbor,
            _controller_channel_commands,
            _controller_channel_messages,
        ) = util::build_browser();

        browser_client
            .topology()
            .set_node_type(21, "server".to_string());

        let st_request = SimControllerCommand::RequestServerType(21);

        browser_client.handle_sim_controller_packets(Ok(st_request));

        let received_server_type = neighbor.1.recv().unwrap();

        let destination = received_server_type.routing_header.destination();
        assert!(destination.is_some());
        assert!(destination.unwrap() == 21);

        let fragment = match received_server_type.pack_type {
            PacketType::MsgFragment(fragment) => fragment,
            _ => panic!("Unexpected packet type"),
        };

        let request = Assembler::new().add_fragment(fragment, 0);
        assert!(request.is_some());
        let binding = request.unwrap();
        let request = serde_json::from_slice::<BrowserRequestWrapper>(&binding).unwrap();

        assert!(matches!(
            request,
            BrowserRequestWrapper::ServerType(ServerTypeRequest::ServerType)
        ));
    }
}
