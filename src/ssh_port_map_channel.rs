use std::io::{ErrorKind, Read, Write};

use ssh2::*;
use tokio::sync::Mutex;

use super::{to_async, SshSessionError};

pub struct SshPortMapChannel {
    channel: Mutex<Option<Channel>>,
}

impl SshPortMapChannel {
    pub async fn connect(ssh_session: &Session, host: &str, port: u16) -> Result<Self, Error> {
        let channel =
            to_async::await_would_block(|| ssh_session.channel_direct_tcpip(host, port, None))
                .await?;

        let result = Self {
            channel: Mutex::new(Some(channel)),
        };
        Ok(result)
    }

    pub async fn write_to_channel(&self, data: &[u8]) -> Result<(), SshSessionError> {
        let mut write_access = self.channel.lock().await;
        match write_access.as_mut() {
            Some(channel) => {
                channel.write_all(data)?;
                Ok(())
            }
            None => {
                return Err(SshSessionError::SshSessionIsNotActive);
            }
        }
    }

    pub async fn read_from_channel(&self, data: &mut [u8]) -> Result<usize, SshSessionError> {
        loop {
            let result = {
                let mut write_access = self.channel.lock().await;
                match write_access.as_mut() {
                    Some(channel) => channel.read(data),
                    None => {
                        return Err(SshSessionError::SshSessionIsNotActive);
                    }
                }
            };

            match result {
                Ok(size) => return Ok(size),
                Err(err) => {
                    if would_block_std_error(&err) {
                        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                    } else {
                        return Err(SshSessionError::StdIoStreamError(err));
                    }
                }
            }
        }
    }

    pub async fn close(&self) {
        let mut write_access = self.channel.lock().await;

        if let Some(mut channel) = write_access.take() {
            let _ = channel.close();
        }
    }
}

fn would_block_std_error(e: &std::io::Error) -> bool {
    match e.kind() {
        ErrorKind::WouldBlock => true,
        _ => false,
    }
}
