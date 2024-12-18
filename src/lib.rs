pub mod assembler;
pub mod chat_client;
pub mod client;
pub mod message;
pub mod server;
pub mod topology;

#[cfg(test)]
mod tests {
    mod flooding_test;
    mod list_test;
    mod register_test;
    mod routing_test;
    mod send_message_test;
}
