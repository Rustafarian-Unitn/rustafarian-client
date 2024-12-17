#[cfg(test)]
pub mod client_list_test {
    use std::{collections::HashMap, ops::MulAssign};

    use crossbeam_channel::{unbounded, Receiver, Sender};
    use wg_2024::packet::{Fragment, Packet, PacketType};

    use crate::{
        assembler::{
            assembler::{self, Assembler},
            disassembler::{self, Disassembler},
        },
        chat_client::ChatClient,
        client::Client,
        message::{ChatRequest, ChatResponse},
    };

    #[test]
    fn test_receive_client_list() {
        let neighbor: (Sender<Packet>, Receiver<Packet>) = unbounded();
        let mut neighbors = HashMap::new();
        neighbors.insert(2 as u8, neighbor.0);
        let channel: (Sender<Packet>, Receiver<Packet>) = unbounded();
        let client_id = 1;

        let mut chat_client = ChatClient::new(client_id, neighbors, channel.1, unbounded().1, unbounded().0);

        let client_list_response = ChatResponse::ClientList([11, 12].to_vec());

        let mut disassembler = Disassembler::new();
        let fragments = disassembler.disassemble_message(
            serde_json::to_string(&client_list_response)
                .unwrap()
                .as_bytes()
                .to_vec(),
            0,
        );
        let fake_packet = Packet {
            pack_type: PacketType::MsgFragment(fragments[0].clone()),
            session_id: rand::random(),
            routing_header: wg_2024::network::SourceRoutingHeader {
                hop_index: 1,
                hops: vec![],
            },
        };

        chat_client.handle_drone_packets(Ok(fake_packet));

        assert_eq!(chat_client.get_client_list(), &vec![11, 12]);
    }
}
