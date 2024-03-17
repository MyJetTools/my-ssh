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
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let self_mut = self.get_mut();
        match self_mut.channel.read(buf.initialize_unfilled()) {
            Ok(size) => {
                buf.advance(size);
                Poll::Ready(Ok(()))
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Err(e) => Poll::Ready(Err(e)),
        }
    }
}

impl tokio::io::AsyncWrite for SshAsyncChannel {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        let self_mut = self.get_mut();
        match self_mut.channel.write(buf) {
            Ok(size) => Poll::Ready(Ok(size)),
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Err(e) => Poll::Ready(Err(e)),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(self.get_mut().channel.flush())
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}
