use std::path::Path;

use async_ssh2_lite::{AsyncChannel, TokioTcpStream};
use futures::AsyncReadExt;
use rust_extensions::StrOrString;
use tokio::sync::Mutex;

use crate::{SshAsyncChannel, SshAsyncSession, SshSessionError};

pub struct SshSessionWrapper {
    ssh_session: SshAsyncSession,
    channel: Mutex<Option<AsyncChannel<TokioTcpStream>>>,
}
impl SshSessionWrapper {
    pub fn new(ssh_session: SshAsyncSession) -> Self {
        Self {
            ssh_session,
            channel: Mutex::new(None),
        }
    }
    pub async fn download_remote_file<'s>(
        &self,
        path: StrOrString<'s>,
    ) -> Result<Vec<u8>, SshSessionError> {
        let (mut remote_file, _) = self.ssh_session.scp_recv(Path::new(path.as_str())).await?;

        let mut contents = Vec::new();
        remote_file.read_to_end(&mut contents).await?;
        remote_file.send_eof().await?;
        remote_file.wait_eof().await?;
        remote_file.close().await?;
        remote_file.wait_close().await?;

        Ok(contents)
    }

    pub async fn channel_direct_tcp_ip(
        &self,
        host: &str,
        port: u16,
    ) -> Result<SshAsyncChannel, SshSessionError> {
        let result = self
            .ssh_session
            .channel_direct_tcpip(host, port, None)
            .await?;

        Ok(result)
    }

    pub async fn execute_command(&self, command: &str) -> Result<String, SshSessionError> {
        let mut channel_access = self.channel.lock().await;

        if channel_access.is_none() {
            let channel = self.ssh_session.channel_session().await?;
            channel_access.replace(channel);
        }

        let channel = channel_access.as_mut().unwrap();
        channel.exec(command).await?;

        let mut result = String::new();
        channel.read_to_string(&mut result).await?;

        Ok(result)
    }

    pub async fn disconnect(&self, description: &str) {
        let _ = self.ssh_session.disconnect(None, description, None).await;
    }
}
