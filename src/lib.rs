pub mod chat_client;
pub mod client;
pub mod message;
pub mod topology;
pub mod server;
pub mod assembler;

#[cfg(test)]
mod tests {
    mod routing_test;
    mod flooding;
    mod send_message;
}