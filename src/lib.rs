pub mod chat_client;
pub mod client;
pub mod message;
pub mod routing;
pub mod server;

#[cfg(test)]
mod tests {
    mod routing_test;
    mod flooding;
}