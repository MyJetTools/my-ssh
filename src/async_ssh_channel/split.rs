use std::sync::{Arc, Mutex};

use super::*;

pub fn split(channel: ssh2::Channel) -> (SshChannelReadHalf, SshChannelWriteHalf) {
    let channel_wrapper = Arc::new(Mutex::new(Some(channel)));
    (
        SshChannelReadHalf::new(channel_wrapper.clone()),
        SshChannelWriteHalf::new(channel_wrapper),
    )
}
