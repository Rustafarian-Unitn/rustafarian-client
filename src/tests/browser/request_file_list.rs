#[cfg(test)]
pub mod request_file_list_tests {
    use rustafarian_shared::{
        assembler::disassembler::Disassembler,
        messages::{
            browser_messages::{
                BrowserRequest, BrowserRequestWrapper, BrowserResponse, BrowserResponseWrapper,
            },
            commander_messages::{SimControllerMessage, SimControllerResponseWrapper},
            general_messages::{DroneSend, ServerType, ServerTypeResponse},
        },
    };
    use wg_2024::{
        network::SourceRoutingHeader,
        packet::{Packet, PacketType},
    };

    use crate::{client::Client, tests::util::build_browser};

    #[test]
    fn request_file_list() {
        let (mut browser_client, neighbor, _, _) = build_browser();
        browser_client.request_file_list(21);

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
    fn file_list_response_text() {
        let (mut browser_client, _neighbor, _sim_controller_commands, sim_controller_response) =
            build_browser();

        // First, get the server type

        let server_type_response =
            BrowserResponseWrapper::ServerType(ServerTypeResponse::ServerType(ServerType::Text));

        let server_type_response_json = server_type_response.stringify();

        let disassembled = Disassembler::new()
            .disassemble_message(server_type_response_json.as_bytes().to_vec(), 0);

        let packet = Packet {
            routing_header: SourceRoutingHeader::new(vec![21, 2, 1], 1),
            session_id: 0,
            pack_type: PacketType::MsgFragment(disassembled.get(0).unwrap().clone()),
        };

        browser_client.on_drone_packet_received(Ok(packet));

        let file_list_response =
            BrowserResponseWrapper::Chat(BrowserResponse::FileList(vec![1, 2, 3, 4, 5]));

        let file_list_response_json = file_list_response.stringify();

        let disassembled =
            Disassembler::new().disassemble_message(file_list_response_json.as_bytes().to_vec(), 0);

        let packet = Packet {
            routing_header: SourceRoutingHeader::new(vec![21, 2, 1], 1),
            session_id: 0,
            pack_type: PacketType::MsgFragment(disassembled.get(0).unwrap().clone()),
        };

        browser_client.on_drone_packet_received(Ok(packet));

        assert_eq!(
            browser_client.get_available_text_files().get(&21).unwrap(),
            &vec![1, 2, 3, 4, 5]
        );

        // After that, it receives a ServerType message
        let _sim_controller_message = sim_controller_response.1.recv().unwrap();

        // Finally, it receives the response for the FileList
        let sim_controller_message = sim_controller_response.1.recv().unwrap();

        match sim_controller_message {
            SimControllerResponseWrapper::Message(message) => match message {
                SimControllerMessage::FileListResponse(21, files) => {
                    assert_eq!(files, vec![1, 2, 3, 4, 5]);
                }
                _ => panic!("Unexpected message"),
            },
            _ => panic!("Unexpected message"),
        }
    }

    #[test]
    fn file_list_response_media() {
        let (mut browser_client, _neighbor, _sim_controller_commands, sim_controller_response) =
            build_browser();

        // First, get the server type

        let server_type_response =
            BrowserResponseWrapper::ServerType(ServerTypeResponse::ServerType(ServerType::Media));

        let server_type_response_json = server_type_response.stringify();

        let disassembled = Disassembler::new()
            .disassemble_message(server_type_response_json.as_bytes().to_vec(), 0);

        let packet = Packet {
            routing_header: SourceRoutingHeader::new(vec![21, 2, 1], 1),
            session_id: 0,
            pack_type: PacketType::MsgFragment(disassembled.get(0).unwrap().clone()),
        };

        browser_client.on_drone_packet_received(Ok(packet));

        let file_list_response =
            BrowserResponseWrapper::Chat(BrowserResponse::FileList(vec![1, 2, 3, 4, 5]));

        let file_list_response_json = file_list_response.stringify();

        let disassembled =
            Disassembler::new().disassemble_message(file_list_response_json.as_bytes().to_vec(), 0);

        let packet = Packet {
            routing_header: SourceRoutingHeader::new(vec![21, 2, 1], 1),
            session_id: 0,
            pack_type: PacketType::MsgFragment(disassembled.get(0).unwrap().clone()),
        };

        browser_client.on_drone_packet_received(Ok(packet));

        assert_eq!(
            browser_client.get_available_media_files().get(&21).unwrap(),
            &vec![1, 2, 3, 4, 5]
        );

        // After that, it receives a ServerType message
        let _sim_controller_message = sim_controller_response.1.recv().unwrap();

        println!("{:?}", _sim_controller_message);

        // Finally, it receives the response for the FileList
        let sim_controller_message = sim_controller_response.1.recv().unwrap();

        println!("{:?}", sim_controller_message);

        match sim_controller_message {
            SimControllerResponseWrapper::Message(message) => match message {
                SimControllerMessage::FileListResponse(21, files) => {
                    assert_eq!(files, vec![1, 2, 3, 4, 5]);
                }
                _ => panic!("Unexpected message"),
            },
            _ => panic!("Unexpected message"),
        }
    }

    #[test]
    fn unknown_file_list() {
        let (mut browser_client, _neighbor, _sim_controller_commands, _sim_controller_response) =
            build_browser();

        // Test without sending the server type first, the client shouldn't recognize it

        let file_list_response =
            BrowserResponseWrapper::Chat(BrowserResponse::FileList(vec![1, 2, 3, 4, 5]));

        let file_list_response_json = file_list_response.stringify();

        let disassembled =
            Disassembler::new().disassemble_message(file_list_response_json.as_bytes().to_vec(), 0);

        let packet = Packet {
            routing_header: SourceRoutingHeader::new(vec![21, 2, 1], 1),
            session_id: 0,
            pack_type: PacketType::MsgFragment(disassembled.get(0).unwrap().clone()),
        };

        browser_client.on_drone_packet_received(Ok(packet));

        assert!(
            !browser_client.get_available_text_files().contains_key(&21)
        );
    }
}
