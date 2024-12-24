#[cfg(test)]
pub mod test_channels {
    use crate::tests::util;

    #[test]
    fn test_controller_channels() {
        let (_chat_client, _neighbor, _controller_channel_commands, _controller_channel_messages) =
            util::build_client();
    }
}
