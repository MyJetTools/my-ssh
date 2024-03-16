use ssh2::Channel;
use std::io::{self, Write};
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use std::task::{Context, Poll};
use tokio::io::Result;

pub struct SshChannelWriteHalf(Arc<Mutex<Option<Channel>>>);

impl SshChannelWriteHalf {
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

impl tokio::io::AsyncWrite for SshChannelWriteHalf {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize>> {
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
                return Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::Other,
                    "SSH channel not available",
                )))
            }
        };

        match channel.write(buf) {
            Ok(size) => Poll::Ready(Ok(size)),
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Err(e) => Poll::Ready(Err(e)),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
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
                return Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::Other,
                    "SSH channel not available",
                )))
            }
        };

        match channel.flush() {
            Ok(()) => Poll::Ready(Ok(())),
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Err(e) => Poll::Ready(Err(e)),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<()>> {
        // Typically, you'd want to send some kind of shutdown signal or close the channel.
        // For ssh2::Channel, we can mimic a close operation.
        let mut ssh_channel_guard = self.0.lock().unwrap();

        if let Some(mut channel) = ssh_channel_guard.take() {
            // Attempt to close the channel
            super::ssh_channel_utils::shutdown_ssh_channel(&mut channel);
        }

        Poll::Ready(Ok(()))
    }
}
