# Rustafarian Client: Chat

## Getting Started

Can be included with:

```toml
rustafarian-client = { git="https://github.com/Rustafarian-Unitn/rustafarian-client" }
```

The two structs to instantiate are:

- ```src/browser_client.rs``: Browser client, used to communicate with Browser Servers;
- ```src/chat_client.rs```: Chat client, used to communicate with Client Servers;

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

According to the tool Tarpaulin (`./tarpaulin-report.html`), at the time of writing (02/01/2025), the unit test coverage is as follows:

- `src/client.rs`: 196/222 (88.29%)
- `src/browser_client.rs`: 158/168 (94.05%)
- `src/chat_client.rs`: 152/154 (98.70%)
