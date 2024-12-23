pub mod chat_client;
pub mod client;

#[cfg(test)]
mod tests {
    mod controller_test;
    mod flooding_test;
    mod list_test;
    mod register_test;
    mod routing_test;
    mod send_message_test;
    mod ack_test;
    mod nack_test;
    mod flood_req_test;
    mod server_type_test;
}
