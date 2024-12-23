use std::collections::HashMap;

use crossbeam_channel::{unbounded, Receiver, Sender};
use rustafarian_client::{chat_client::ChatClient, client::Client};
use wg_2024::packet::Packet;

fn main() {
    let channel: (Sender<Packet>, Receiver<Packet>) = crossbeam_channel::unbounded();
    let mut chat_client =
        ChatClient::new(1, HashMap::new(), channel.1, unbounded().1, unbounded().0);
    chat_client.run(u64::MAX);

    loop {
        // Take input
        println!("Choose an option:");
        println!("1. Flood request");
        println!("2. Register to server");
        println!("3. List of clients");
        println!("4. Send message");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        let input: u8 = input.trim().parse().unwrap();
        if input == 1 {
            chat_client.send_flood_request();
        } else if input == 2 {
            chat_client.register(21);
        } else if input == 3 {
            chat_client.get_client_list();
        } else if input == 4 {
            println!("Enter client id:");
            let mut client_id = String::new();
            std::io::stdin().read_line(&mut client_id).unwrap();
            let client_id: u8 = client_id.trim().parse().unwrap();
            println!("Enter message:");
            let mut message = String::new();
            std::io::stdin().read_line(&mut message).unwrap();
            chat_client.send_message(client_id, message);
        }
    }

    // let mut browser_client = BrowserClient::new(2, HashMap::new(), crossbeam_channel::unbounded().1);
    // ...additional code for browser_client...
}
