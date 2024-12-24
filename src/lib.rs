pub mod chat_client;
pub mod client;
pub mod browser_client;

#[cfg(test)]
mod tests {
    mod util;
    
    mod ack_test;
    mod controller_test;
    mod flood_req_test;
    mod flooding_test;
    mod list_test;
    mod nack_test;
    mod register_test;
    mod routing_test;
    mod send_message_test;
    mod server_type_test;
    mod test_channels;
    mod error_tests;
    mod test_running;
}
