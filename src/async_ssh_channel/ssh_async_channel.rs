use std::{
    io::{Read, Write},
    pin::Pin,
    task::{Context, Poll},
};

use super::ReadBuffer;

/*
pub struct SshAsyncInner {
    channel: ssh2::Channel,
    from_ssh_buffer: Option<ReadBuffer>,
    disconnected: bool,
}

impl SshAsyncInner {
    pub fn new(channel: ssh2::Channel) -> Self {
        Self {
            channel,
            from_ssh_buffer: Some(ReadBuffer::new()),
            disconnected: false,
        }
    }
    pub fn disconnect(&mut self) {
        self.disconnected = true;
    }

    pub fn read(&mut self) -> Result<usize, std::io::Error> {
        let mut from_ssh_buffer = self.from_ssh_buffer.take().unwrap();

        let result = if let Some(buffer_to_write) = from_ssh_buffer.get_write_buf() {
            self.channel.read(buffer_to_write)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::WouldBlock,
                "No buffer to write",
            ))
        };

        if let Ok(result) = result {
            from_ssh_buffer.advance(result);
        }

        self.from_ssh_buffer = Some(from_ssh_buffer);

        result
    }
}
 */
pub struct SshAsyncChannel {
    channel: ssh2::Channel,
    read_buffer: ReadBuffer,
}

impl SshAsyncChannel {
    pub fn new(channel: ssh2::Channel, buffer_size: usize) -> Self {
        Self {
            channel,
            read_buffer: ReadBuffer::new(buffer_size),
        }
    }

    pub fn read_to_internal_buffer(&mut self) -> Result<(), std::io::Error> {
        loop {
            let read = if let Some(buffer_to_write) = self.read_buffer.get_write_buf() {
                self.channel.read(buffer_to_write)
            } else {
                return Ok(());
            };

            match read {
                Ok(size) => {
                    if size > 0 {
                        self.read_buffer.advance(size);
                    }
                    return Ok(());
                }
                Err(err) => {
                    if err.kind() == std::io::ErrorKind::WouldBlock {
                        return Ok(());
                    }
                    return Err(err);
                }
            }
        }
    }
}

impl tokio::io::AsyncRead for SshAsyncChannel {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let self_mut = self.get_mut();

        self_mut.read_to_internal_buffer()?;

        let written = self_mut
            .read_buffer
            .write_to_buffer(buf.initialize_unfilled());

        if written > 0 {
            buf.advance(written);
            return Poll::Ready(Ok(()));
        }

        cx.waker().wake_by_ref();
        Poll::Pending
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

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        let self_mut = self.get_mut();

        match self_mut.channel.flush() {
            Ok(()) => Poll::Ready(Ok(())),
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Err(e) => Poll::Ready(Err(e)),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

/*
impl tokio::io::AsyncRead for SshAsyncChannel {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let mut ssh_channel_guard = match self.inner.lock() {
            Ok(guard) => guard,
            Err(_) => {
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
        };

        if ssh_channel_guard.disconnected {
            return Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "SSH channel disconnected",
            )));
        }

        let written_size = ssh_channel_guard
            .from_ssh_buffer
            .as_mut()
            .unwrap()
            .write_to_buffer(buf.initialize_unfilled());

        if written_size == 0 {
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }

        buf.advance(written_size);
        Poll::Ready(Ok(()))
    }
}

impl tokio::io::AsyncWrite for SshAsyncChannel {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        let mut ssh_channel_guard = match self.inner.lock() {
            Ok(guard) => guard,
            Err(_) => {
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
        };

        let result = ssh_channel_guard.channel.write(buf);
        return Poll::Ready(result);

        /*
        if let Some(write_buffer) = ssh_channel_guard.to_ssh_buffer.get_write_buf() {
            let written = if write_buffer.len() < buf.len() {
                let buf_to_copy = &buf[..write_buffer.len()];

                write_buffer.copy_from_slice(buf_to_copy);
                write_buffer.len()
            } else {
                let write_buffer = &mut write_buffer[..buf.len()];
                write_buffer.copy_from_slice(buf);
                write_buffer.len()
            };

            return Poll::Ready(Ok(written));
        }
        cx.waker().wake_by_ref();
        return Poll::Pending;
         */
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        let mut ssh_channel_guard = match self.inner.lock() {
            Ok(guard) => guard,
            Err(_) => {
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
        };

        let result = ssh_channel_guard.channel.flush();
        return Poll::Ready(result);
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        let mut ssh_channel_guard = match self.inner.lock() {
            Ok(guard) => guard,
            Err(_) => {
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
        };

        ssh_channel_guard.disconnect();
        Poll::Ready(Ok(()))
    }
}

async fn read_thread(inner: Arc<Mutex<SshAsyncInner>>) {
    loop {
        let result = {
            let mut write_access = inner.lock().unwrap();
            if write_access.disconnected {
                return;
            }
            write_access.read()
        };

        match result {
            Ok(size) => {
                if size == 0 {
                    return;
                }
            }
            Err(err) => {
                if err.kind() == std::io::ErrorKind::WouldBlock {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                } else {
                    return;
                }
            }
        }
    }
}
*/
