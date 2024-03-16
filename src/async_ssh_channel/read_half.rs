use ssh2::*;
use std::{
    io::Read,
    sync::{Arc, Mutex},
    task::Poll,
};

pub struct SshChannelReadHalf(Arc<Mutex<Option<Channel>>>);

impl SshChannelReadHalf {
    pub fn new(channel_wrapper: Arc<Mutex<Option<Channel>>>) -> Self {
        Self(channel_wrapper)
    }

    pub fn shutdown(&self) {
        let mut write_access = self.0.lock().unwrap();
        if let Some(mut channel) = write_access.take() {
            // Attempt to close the channel
            super::ssh_channel_utils::shutdown_ssh_channel(&mut channel);
        }
    }
}

impl tokio::io::AsyncRead for SshChannelReadHalf {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let mut ssh_channel_guard = match self.0.lock() {
            Ok(guard) => guard,
            Err(_) => {
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
        };

        let channel = match ssh_channel_guard.as_mut() {
            Some(channel) => channel,
            None => {
                return Poll::Ready(Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "SSH channel not available",
                )))
            }
        };

        match channel.read(buf.initialize_unfilled()) {
            Ok(0) => {
                // No more data to read (EOF)
                Poll::Ready(Ok(()))
            }
            Ok(n) => {
                // Data was read, advance the buffer
                buf.advance(n);
                Poll::Ready(Ok(()))
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // No data available yet, register the current task to be notified
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Err(e) => {
                println!("Error: {:?}", e);
                // An actual error occurred
                Poll::Ready(Err(e))
            }
        }
    }
}

/*
fn would_block_std_error(e: &std::io::Error) -> bool {
    match e.kind() {
        ErrorKind::WouldBlock => true,
        _ => false,
    }
}
 */
