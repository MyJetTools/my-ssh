use std::path::Path;

use futures::AsyncReadExt;
use rust_extensions::StrOrString;

use crate::{SshAsyncChannel, SshAsyncSession, SshSessionError};

pub struct SshSessionWrapper {
    ssh_session: SshAsyncSession,
}
impl SshSessionWrapper {
    pub fn new(ssh_session: SshAsyncSession) -> Self {
        Self { ssh_session }
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

    pub async fn execute_command(&self, command: &str) -> Result<(String, i32), SshSessionError> {
        let mut channel = self.ssh_session.channel_session().await?;

        channel.exec(command).await?;

        let mut result = String::new();
        channel.read_to_string(&mut result).await?;

        channel.wait_close().await?;

        Ok((result, channel.exit_status()?))
    }

    pub async fn disconnect(&self, description: &str) {
        let _ = self.ssh_session.disconnect(None, description, None).await;
    }
}
