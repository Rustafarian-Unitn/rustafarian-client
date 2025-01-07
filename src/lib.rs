pub mod browser_client;
pub mod chat_client;
pub mod client;
pub mod utils;

#[cfg(test)]
mod tests {
    mod util;

    mod browser;
    mod chat;
    mod routing_test;
}
