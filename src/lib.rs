pub mod browser_client;
pub mod chat_client;
pub mod client;

#[cfg(test)]
mod tests {
    mod util;

    mod browser;
    mod chat;
    mod routing_test;
}
