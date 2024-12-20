pub mod chat_client;
pub mod client;

#[cfg(test)]
mod tests {
    mod flooding_test;
    mod list_test;
    mod register_test;
    mod routing_test;
    mod send_message_test;
    mod controller_test;
}
