#[cfg(test)]
pub mod request_file_list {
    use rustafarian_shared::{
        assembler::disassembler::Disassembler,
        messages::{
            browser_messages::{
                BrowserRequest, BrowserRequestWrapper, BrowserResponse, BrowserResponseWrapper,
            },
            commander_messages::{
                SimControllerEvent, SimControllerMessage, SimControllerResponseWrapper,
            },
            general_messages::DroneSend,
        },
    };
    use wg_2024::{
        network::SourceRoutingHeader,
        packet::{Packet, PacketType},
    };

    use crate::{client::Client, tests::util::build_browser};

    #[test]
    fn request_media() {
        let (mut browser_client, neighbor, _, _) = build_browser();
        browser_client.request_media_file(1, 21);

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
    fn media_response() {
        let (mut browser_client, _neighbor, _sim_controller_commands, sim_controller_response) =
            build_browser();

        let example_media_file = vec![1, 2, 3, 4, 5];

        let file_list_response = BrowserResponseWrapper::Chat(BrowserResponse::MediaFile(
            111,
            example_media_file.clone(),
        ));

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
            browser_client
                .get_obtained_media_files()
                .get(&(21, 111))
                .unwrap(),
            &example_media_file
        );

        // Sim Controller Packet Sent Event
        let sim_controller_message = sim_controller_response.1.recv().unwrap();

        match sim_controller_message {
            SimControllerResponseWrapper::Event(event) => match event {
                SimControllerEvent::PacketReceived(packet_id) => {
                    assert_eq!(packet_id, 0);
                }
                _ => panic!("Unexpected event"),
            },
            _ => panic!("Unexpected message"),
        }

        // Then tests sim controller message

        let sim_controller_message = sim_controller_response.1.recv().unwrap();

        match sim_controller_message {
            SimControllerResponseWrapper::Message(message) => match message {
                SimControllerMessage::MediaFileResponse(file_id, content) => {
                    assert_eq!(file_id, 111);
                    assert_eq!(content, example_media_file);
                }
                _ => panic!("Unexpected message"),
            },
            _ => panic!("Unexpected message"),
        }
    }
}
