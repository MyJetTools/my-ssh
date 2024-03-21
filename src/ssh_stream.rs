use std::{
    io::{Read, Write},
    task::Poll,
};

use ssh2::*;
use tokio::{io::AsyncReadExt, sync::Mutex};

use super::{to_async, SshSessionError};

pub struct SshStream {
    channel: Mutex<Option<ChannelWrapper>>,
}

impl SshStream {
    pub async fn connect(ssh_session: &Session, host: &str, port: u16) -> Result<Self, Error> {
        let channel =
            to_async::await_would_block(|| ssh_session.channel_direct_tcpip(host, port, None))
                .await?;

        let result = Self {
            channel: Mutex::new(Some(channel.into())),
        };
        Ok(result)
    }

    pub async fn write_to_channel(&self, data: &[u8]) -> Result<(), SshSessionError> {
        let mut write_access = self.channel.lock().await;
        match write_access.as_mut() {
            Some(channel) => {
                channel.0.write_all(data)?;
                Ok(())
            }
            None => {
                return Err(SshSessionError::SshSessionIsNotActive);
            }
        }
    }

    pub async fn read_from_channel(&self, data: &mut [u8]) -> Result<usize, SshSessionError> {
        let mut write_access = self.channel.lock().await;
        match write_access.as_mut() {
            Some(channel) => {
                let result = channel.read(data).await?;
                Ok(result)
            }
            None => {
                return Err(SshSessionError::SshSessionIsNotActive);
            }
        }
    }

    pub async fn shutdown(&self) {
        let mut write_access = self.channel.lock().await;

        if let Some(mut channel) = write_access.take() {
            let _ = channel.0.close();
        }
    }
}
