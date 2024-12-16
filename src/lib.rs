pub mod message;
pub mod server;
pub mod routing;
pub mod client;
pub mod chat_client;

#[cfg(test)]
mod tests {
    mod routing_test;
    mod flooding;
}