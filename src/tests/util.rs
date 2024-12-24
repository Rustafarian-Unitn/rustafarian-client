use std::collections::HashMap;

use crossbeam_channel::{unbounded, Receiver, Sender};
use rustafarian_shared::messages::commander_messages::{
    SimControllerCommand, SimControllerResponseWrapper,
};
use wg_2024::packet::Packet;

use crate::{browser_client::BrowserClient, chat_client::ChatClient, client::Client};

pub(crate) fn build_client() -> (
    ChatClient,
    (Sender<Packet>, Receiver<Packet>),
    (Sender<SimControllerCommand>, Receiver<SimControllerCommand>),
    (
        Sender<SimControllerResponseWrapper>,
        Receiver<SimControllerResponseWrapper>,
    ),
) {
    let neighbor: (Sender<Packet>, Receiver<Packet>) = unbounded();
    let mut neighbors = HashMap::new();
    neighbors.insert(2 as u8, neighbor.0.clone());
    let channel: (Sender<Packet>, Receiver<Packet>) = unbounded();
    let client_id = 1;

    let controller_channel_commands = unbounded();
    let controller_channel_messages = unbounded();

    let mut chat_client = ChatClient::new(
        client_id,
        neighbors,
        channel.1,
        controller_channel_commands.1.clone(),
        controller_channel_messages.0.clone(),
    );

    chat_client.topology().add_node(2);
    chat_client.topology().add_node(21);
    chat_client.topology().add_edge(2, 21);
    chat_client.topology().add_edge(1, 2);

    (
        chat_client,
        neighbor,
        controller_channel_commands,
        controller_channel_messages,
    )
}

pub(crate) fn build_browser() -> (
    BrowserClient,
    (Sender<Packet>, Receiver<Packet>),
    (Sender<SimControllerCommand>, Receiver<SimControllerCommand>),
    (
        Sender<SimControllerResponseWrapper>,
        Receiver<SimControllerResponseWrapper>,
    ),
) {
    let neighbor: (Sender<Packet>, Receiver<Packet>) = unbounded();
    let mut neighbors = HashMap::new();
    neighbors.insert(2 as u8, neighbor.0.clone());
    let channel: (Sender<Packet>, Receiver<Packet>) = unbounded();
    let client_id = 1;

    let controller_channel_commands = unbounded();
    let controller_channel_messages = unbounded();

    let mut browser_client = BrowserClient::new(
        client_id,
        neighbors,
        channel.1,
        controller_channel_commands.1.clone(),
        controller_channel_messages.0.clone(),
    );

    browser_client.topology().add_node(2);
    browser_client.topology().add_node(21);
    browser_client.topology().add_edge(2, 21);
    browser_client.topology().add_edge(1, 2);

    (
        browser_client,
        neighbor,
        controller_channel_commands,
        controller_channel_messages,
    )
}
