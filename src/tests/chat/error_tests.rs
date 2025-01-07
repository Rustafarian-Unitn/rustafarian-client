#[cfg(test)]
pub mod error_tests {

    use std::collections::HashMap;

    use crossbeam_channel::{unbounded, Receiver, RecvError, Sender};
    use rustafarian_shared::messages::chat_messages::{ChatResponse, ChatResponseWrapper};
    use wg_2024::network::SourceRoutingHeader;
    use wg_2024::packet::{Fragment, Nack, Packet, PacketType};

    use crate::chat_client::ChatClient;
    use crate::client::Client;
    use crate::tests::util;

    #[test]
    pub fn compose_message_error() {
        let (chat_client, _neighbor, _controller_channel_commands, _controller_channel_messages) =
            util::build_client();

        let result = chat_client.compose_message(21, 0, "asd".to_string());
        assert!(matches!(result, Err(_)));

        let message = ChatResponseWrapper::Chat(ChatResponse::MessageSent);
        let message_json = serde_json::to_string(&message).unwrap();
        let result = chat_client.compose_message(21, 0, message_json);
        assert!(matches!(result, Ok(_)));
    }

    #[test]
    #[should_panic]
    pub fn on_text_response_error() {
        let (
            mut chat_client,
            _neighbor,
            _controller_channel_commands,
            _controller_channel_messages,
        ) = util::build_client();

        chat_client.on_text_response_arrived(21, 0, "asd".to_string());
    }

    #[test]
    #[should_panic]
    pub fn no_packet_id() {
        let (
            mut chat_client,
            _neighbor,
            _controller_channel_commands,
            _controller_channel_messages,
        ) = util::build_client();

        let packet = Packet {
            pack_type: PacketType::Nack(Nack {
                nack_type: wg_2024::packet::NackType::ErrorInRouting(2),
                fragment_index: 0,
            }),
            routing_header: SourceRoutingHeader {
                hops: vec![21, 2, 1],
                hop_index: 1,
            },
            session_id: 999,
        };

        let nack = Nack {
            nack_type: wg_2024::packet::NackType::ErrorInRouting(2),
            fragment_index: 0,
        };

        chat_client.on_nack_received(packet, nack);
    }

    #[test]
    #[should_panic]
    pub fn packet_len() {
        let (
            mut chat_client,
            _neighbor,
            _controller_channel_commands,
            _controller_channel_messages,
        ) = util::build_client();

        let packet = Packet {
            pack_type: PacketType::Nack(Nack {
                nack_type: wg_2024::packet::NackType::ErrorInRouting(2),
                fragment_index: 10,
            }),
            routing_header: SourceRoutingHeader {
                hops: vec![21, 2, 1],
                hop_index: 1,
            },
            session_id: 0,
        };

        let nack = Nack {
            nack_type: wg_2024::packet::NackType::ErrorInRouting(2),
            fragment_index: 999,
        };
        chat_client.sent_packets().insert(0, vec![]);
        chat_client
            .sent_packets()
            .get_mut(&0)
            .unwrap()
            .push(Packet {
                pack_type: PacketType::MsgFragment(Fragment {
                    fragment_index: 0,
                    total_n_fragments: 1,
                    length: 10,
                    data: [0; 128],
                }),
                routing_header: SourceRoutingHeader {
                    hops: vec![1, 2, 21],
                    hop_index: 1,
                },
                session_id: 0,
            });

        chat_client.on_nack_received(packet, nack);
    }

    #[test]
    #[should_panic]
    fn packet_error() {
        let (
            mut chat_client,
            _neighbor,
            _controller_channel_commands,
            _controller_channel_messages,
        ) = util::build_client();

        chat_client.on_drone_packet_received(Err(RecvError {}));
    }

    #[test]
    #[should_panic]
    fn sim_controller_error() {
        let (
            mut chat_client,
            _neighbor,
            _controller_channel_commands,
            _controller_channel_messages,
        ) = util::build_client();

        chat_client.handle_sim_controller_packets(Err(RecvError {}));
    }

    #[test]
    fn packet_no_route() {
        let (
            mut chat_client,
            _neighbor,
            _controller_channel_commands,
            _controller_channel_messages,
        ) = util::build_client();

        chat_client.topology().remove_node(21);

        let packet = Packet {
            pack_type: PacketType::Nack(Nack {
                nack_type: wg_2024::packet::NackType::ErrorInRouting(2),
                fragment_index: 0,
            }),
            routing_header: chat_client.topology().get_routing_header(1, 21),
            session_id: 0,
        };

        chat_client.send_packet(packet, 21);

        assert!(chat_client.packets_to_send().contains_key(&21));
    }

    #[test]
    #[should_panic]
    fn packet_send_error() {
        let neighbors = HashMap::new();
        let channel: (Sender<Packet>, Receiver<Packet>) = unbounded();
        let controller_channel_commands = unbounded();
        let controller_channel_messages = unbounded();

        let mut chat_client = ChatClient::new(
            1,
            neighbors,
            channel.1,
            controller_channel_commands.1.clone(),
            controller_channel_messages.0.clone(),
            false,
        );

        chat_client.topology().add_node(2);
        chat_client.topology().add_node(21);
        chat_client.topology().add_edge(2, 21);
        chat_client.topology().add_edge(1, 2);

        let packet = Packet {
            pack_type: PacketType::Nack(Nack {
                nack_type: wg_2024::packet::NackType::ErrorInRouting(2),
                fragment_index: 0,
            }),
            routing_header: chat_client.topology().get_routing_header(1, 21),
            session_id: 0,
        };

        chat_client.send_packet(packet, 21);
    }
}
