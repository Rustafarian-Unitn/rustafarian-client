use std::collections::HashMap;

use crossbeam_channel::{Receiver, Sender};
use rustafarian_client::{chat_client::ChatClient, client::Client};
use wg_2024::packet::Packet;



fn main() {
    let mut channel: (Sender<Packet>, Receiver<Packet>) = crossbeam_channel::unbounded();
    let mut chat_client = ChatClient::new(1, HashMap::new(), channel.1);
    chat_client.run();

    while true {

    }

    // let mut browser_client = BrowserClient::new(2, HashMap::new(), crossbeam_channel::unbounded().1);
    // ...additional code for browser_client...
}