#[cfg(test)]
pub mod request_type_tests {
    use std::collections::HashMap;

    use rustafarian_shared::{
        assembler::{assembler::Assembler, disassembler::Disassembler},
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
    fn request_text() {
        let (mut browser_client, neighbor, _, _) = build_browser();
        browser_client.request_text_file(1, 21);

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
    fn text_response() {
        let (mut browser_client, _neighbor, _sim_controller_commands, sim_controller_response) =
            build_browser();

        let example_text_file = String::from("Hello, world!");

        let file_list_response =
            BrowserResponseWrapper::Chat(BrowserResponse::TextFile(222, example_text_file.clone()));

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
                .get_obtained_text_files()
                .get(&(21, 222))
                .unwrap(),
            &example_text_file
        );

        // Then tests sim controller message

        let sim_controller_message = sim_controller_response.1.recv().unwrap();

        match sim_controller_message {
            SimControllerResponseWrapper::Message(message) => match message {
                SimControllerMessage::TextFileResponse(file_id, content) => {
                    assert_eq!(file_id, 222);
                    assert_eq!(content, example_text_file);
                }
                _ => panic!("Unexpected message"),
            },
            _ => panic!("Unexpected message"),
        }
    }

    #[test]
    fn text_with_references() {
        let (mut browser_client, neighbor, _sim_controller_commands, _sim_controller_response) =
            build_browser();

        let server_type_response =
            BrowserResponseWrapper::ServerType(ServerTypeResponse::ServerType(ServerType::Media));
        browser_client.handle_response(server_type_response, 22);

        let server_type_response =
            BrowserResponseWrapper::ServerType(ServerTypeResponse::ServerType(ServerType::Text));
        browser_client.handle_response(server_type_response, 21);

        browser_client.topology().add_edge(2, 22);

        let text_with_ref = String::from("ref=1\nasd");
        let response = BrowserResponseWrapper::Chat(BrowserResponse::TextFile(1, text_with_ref));

        browser_client.handle_response(response, 21);

        // I expect the browser to send a request for the media file referenced in the text file.
        let media_request_msg = neighbor.1.recv().unwrap();
        let fragment = match media_request_msg.pack_type {
            PacketType::MsgFragment(fragment) => fragment,
            _ => panic!("Unexpected packet type"),
        };

        assert_eq!(fragment.fragment_index, 0);

        let request = Assembler::new().add_fragment(fragment, 0);
        assert!(request.is_some());
        let binding = request.unwrap();
        let request = std::str::from_utf8(&binding).unwrap();
        let request = serde_json::from_str::<BrowserRequestWrapper>(request).unwrap();
        assert!(matches!(
            request,
            BrowserRequestWrapper::Chat(BrowserRequest::MediaFileRequest(1))
        ));
    }

    #[test]
    fn text_with_references_cached() {
        let (mut browser_client, _neighbor, _sim_controller_commands, sim_controller_response) =
            build_browser();

        let server_type_response =
            BrowserResponseWrapper::ServerType(ServerTypeResponse::ServerType(ServerType::Media));
        browser_client.handle_response(server_type_response, 22);

        let server_type_response =
            BrowserResponseWrapper::ServerType(ServerTypeResponse::ServerType(ServerType::Text));
        browser_client.handle_response(server_type_response, 21);

        browser_client.topology().add_edge(2, 22);

        // Put the media in the obtained_media, so that it doesn't have to request it
        browser_client
            .get_obtained_media_files()
            .insert(1, vec![1, 2, 3]);

        let text_with_ref = String::from("ref=1\nasd");
        let response =
            BrowserResponseWrapper::Chat(BrowserResponse::TextFile(1, text_with_ref.clone()));

        browser_client.handle_response(response, 21);

        // First, it receives two ServerTypeResponses
        let _sim_controller_message = sim_controller_response.1.recv().unwrap();
        let _sim_controller_message = sim_controller_response.1.recv().unwrap();

        // The client should send the text file to the sim controller with the reference

        let sim_controller_message = sim_controller_response.1.recv().unwrap();
        println!("{:?}", sim_controller_message);
        assert!(matches!(
            sim_controller_message,
            SimControllerResponseWrapper::Message(SimControllerMessage::TextWithReferences(
                1,
                _,
                _
            ))
        ));
        let text =
            match sim_controller_message.clone() {
                SimControllerResponseWrapper::Message(
                    SimControllerMessage::TextWithReferences(_, text, _),
                ) => text,
                _ => panic!("Unexpected message"),
            };
        let media =
            match sim_controller_message {
                SimControllerResponseWrapper::Message(
                    SimControllerMessage::TextWithReferences(_, _, media),
                ) => media,
                _ => panic!("Unexpected message"),
            };
        assert_eq!(text, text_with_ref);
        let mut medias = HashMap::new();
        medias.insert(1, vec![1, 2, 3]);
        assert_eq!(media, medias);
    }
}
