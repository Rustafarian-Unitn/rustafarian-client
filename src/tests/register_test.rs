#[cfg(test)]
pub mod register_test {
    use std::{collections::HashMap, ops::MulAssign};

    use crossbeam_channel::{unbounded, Receiver, Sender};
    use wg_2024::packet::{Fragment, Packet, PacketType};

    use crate::{assembler::{assembler::{self, Assembler}, disassembler}, chat_client::ChatClient, client::Client, message::ChatRequest};

    #[test]
    fn simple_register() {
        let neighbor: (Sender<Packet>, Receiver<Packet>) = unbounded();
        let mut neighbors = HashMap::new();
        neighbors.insert(2 as u8, neighbor.0);
        let channel: (Sender<Packet>, Receiver<Packet>) = unbounded();
        let client_id = 1;

        let mut chat_client = ChatClient::new(client_id, neighbors, channel.1, unbounded().1, unbounded().0);

        let register_request = ChatRequest::Register(client_id);
        let register_serialized = serde_json::to_string(&register_request).unwrap();

        // let mut fragments = disassembler.disassemble_message(register_serialized.as_bytes().to_vec(), 0);

        chat_client.topology().add_node(2);
        chat_client.topology().add_node(21);
        chat_client.topology().add_edge(2, 21);
        chat_client.topology().add_edge(1, 2);

        chat_client.send_message(21, register_serialized.clone());

        let expected_fragment = Fragment { fragment_index: 0, total_n_fragments: 1, length: 14, data: [123, 34, 82, 101, 103, 105, 115, 116, 101, 114, 34, 58, 49, 125, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0] };

        let received_fragment = neighbor.1.recv().unwrap();

        assert_eq!(received_fragment.clone().pack_type, PacketType::MsgFragment(expected_fragment));

        let mut assembler = Assembler::new();

        let fragment = match received_fragment.pack_type {
            PacketType::MsgFragment(fragment) => fragment,
            _ => panic!("Packet type should be MsgFragment")
        };

        let reassembled_message = assembler.add_fragment(fragment, received_fragment.session_id);

        assert_eq!(reassembled_message.unwrap(), register_serialized.as_bytes().to_vec());
    }

}