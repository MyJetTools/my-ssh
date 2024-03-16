use std::{
    io::{Read, Write},
    pin::Pin,
    task::{Context, Poll},
};

pub struct SshAsyncChannel {
    channel: ssh2::Channel,
}

impl SshAsyncChannel {
    pub fn new(channel: ssh2::Channel) -> Self {
        SshAsyncChannel { channel }
    }
}

impl tokio::io::AsyncRead for SshAsyncChannel {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match self.channel.read(buf.initialize_unfilled()) {
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

impl tokio::io::AsyncWrite for SshAsyncChannel {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        match self.channel.write(buf) {
            Ok(size) => Poll::Ready(Ok(size)),
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Err(e) => Poll::Ready(Err(e)),
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.channel.flush() {
            Ok(()) => Poll::Ready(Ok(())),
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Err(e) => Poll::Ready(Err(e)),
        }
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        // Typically, you'd want to send some kind of shutdown signal or close the channel.
        // For ssh2::Channel, we can mimic a close operation.
        super::ssh_channel_utils::shutdown_ssh_channel(&mut self.channel);
        Poll::Ready(Ok(()))
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[std::io::IoSlice<'_>],
    ) -> Poll<Result<usize, std::io::Error>> {
        let buf = bufs
            .iter()
            .find(|b| !b.is_empty())
            .map_or(&[][..], |b| &**b);
        self.poll_write(cx, buf)
    }

    fn is_write_vectored(&self) -> bool {
        false
    }
}
