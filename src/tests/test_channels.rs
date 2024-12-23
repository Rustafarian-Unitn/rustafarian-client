#[cfg(test)]
pub mod test_channels {
    use std::collections::HashMap;

    use crossbeam_channel::{unbounded, Receiver, Sender};
    use wg_2024::packet::Packet;

    use crate::chat_client::ChatClient;
    use crate::client::Client;

    #[test]
    fn test_controller_channels() {
        let neighbor: (Sender<Packet>, Receiver<Packet>) = unbounded();
        let mut neighbors = HashMap::new();
        neighbors.insert(2 as u8, neighbor.0);
        let channel: (Sender<Packet>, Receiver<Packet>) = unbounded();
        let client_id = 1;

        let mut chat_client = ChatClient::new(
            client_id,
            neighbors,
            channel.1,
            unbounded().1,
            unbounded().0,
        );

        chat_client.topology().add_node(2);
        chat_client.topology().add_node(21);
        chat_client.topology().add_edge(2, 21);
        chat_client.topology().add_edge(1, 2);
    }
}
