# Rustafarian Client

This is the client-side for the project for the course "Advanced Programming", at UniTN (University of Trento), a.y. 2024/2025.

The projects consists of other repositories as well:

- [rustafarian-drone](https://github.com/Rustafarian-Unitn/rustafarian-drone): The drone, used to forward messages and handle the flood requests;
- [rustafarian-controller](https://github.com/Rustafarian-Unitn/rustafarian-controller): The backend for the Simulation Controller, used to initialize the network;
- [rustafarian-front-end](https://github.com/Rustafarian-Unitn/rustafarian-front-end): The frontend for the Simulation Controller, shows the network and can send commands to the nodes;
- [rustafarian-content-server](https://github.com/Rustafarian-Unitn/rustafarian-content_server): The content server, used to communicate with the Browser Client;
- [rustafarian-chat-server](https://github.com/Rustafarian-Unitn/rustafarian-chat-server): The chat server, used to communicate between Chat Clients;

## Getting Started

Can be included with:

```toml
rustafarian-client = { git="https://github.com/Rustafarian-Unitn/rustafarian-client" }
```

The two structs to instantiate are:

- `src/browser_client.rs`: Browser client, used to communicate with Browser Servers;
- `src/chat_client.rs`: Chat client, used to communicate with Client Servers;

## Structure

There are two parts, the chat and the browser.

The constructor for the chat client is as follows:

```rust
pub fn new(
    client_id: u8,
    senders: HashMap<u8, Sender<Packet>>,
    receiver: Receiver<Packet>,
    sim_controller_receiver: Receiver<SimControllerCommand>,
    sim_controller_sender: Sender<SimControllerResponseWrapper>,
) -> Self
```

The arguments are:

- `client_id`: The ID for the client, it's a u8 and needs to be unique in the network;
- `senders`: the map containing the neighbors for the client, the key is the ID of the node, the value is the crossbeam channel used to send messages;
- `receiver`: the crossbeam channel used to receive messages from others;
- `sim_controller_receiver`: the crossbeam channel used to receive messages from the Simulation Controller;
- `sim_controller_sender`. the crossbeam channel used to receive send messages to the Simulation Controller;

## Testing

Rigorous unit testing was executed to ensure that the client worked correctly. All the tests are located in the `./src/tests`, and are divided between chat and browser.
The integration testing was done in the `rustafarian-controller` repository.

According to the tool [Tarpaulin](https://github.com/xd009642/tarpaulin) at the time of writing (19/01/2025), the unit test coverage is as follows:

- `src/client.rs`: 229/308 (74.35%) 
- `src/browser_client.rs`: 259/293 (88.40%) 
- `src/chat_client.rs`: 184/189 (97.35%) 
