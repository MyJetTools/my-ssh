use std::sync::Arc;

use russh::ChannelMsg;
use russh_sftp::client::fs::File;
use russh_sftp::client::SftpSession;
use russh_sftp::protocol::{FileAttributes, OpenFlags};
use tokio::sync::Mutex;

use crate::{RemoteProcess, SshAsyncChannel, SshAsyncSession, SshSessionError};

pub struct SshSessionWrapper {
    ssh_session: SshAsyncSession,
    sftp: Mutex<Option<Arc<SftpSession>>>,
}

impl SshSessionWrapper {
    pub fn new(ssh_session: SshAsyncSession) -> Self {
        Self {
            ssh_session,
            sftp: Mutex::new(None),
        }
    }

    pub fn is_closed(&self) -> bool {
        self.ssh_session.is_closed()
    }

    async fn sftp(&self) -> Result<Arc<SftpSession>, SshSessionError> {
        let mut guard = self.sftp.lock().await;

        if let Some(s) = guard.as_ref() {
            return Ok(s.clone());
        }

        let channel = self.ssh_session.channel_open_session().await?;
        channel.request_subsystem(true, "sftp").await?;
        let sftp = SftpSession::new(channel.into_stream()).await?;
        let arc = Arc::new(sftp);
        *guard = Some(arc.clone());
        Ok(arc)
    }

    pub async fn mkdir_recursive(&self, path: String) -> Result<(), SshSessionError> {
        let sftp = self.sftp().await?;
        let absolute = path.starts_with('/');
        let mut current = if absolute {
            String::from("/")
        } else {
            String::new()
        };

        for component in path.split('/').filter(|c| !c.is_empty()) {
            if !current.is_empty() && !current.ends_with('/') {
                current.push('/');
            }
            current.push_str(component);

            if !sftp.try_exists(current.as_str()).await? {
                sftp.create_dir(current.as_str()).await?;
            }
        }

        Ok(())
    }

    pub async fn open_file(
        &self,
        path: String,
        flags: OpenFlags,
        mode: Option<u32>,
    ) -> Result<File, SshSessionError> {
        let sftp = self.sftp().await?;
        let file = sftp.open_with_flags(path.as_str(), flags).await?;

        if let Some(mode) = mode {
            let mut attrs = FileAttributes::default();
            attrs.permissions = Some(mode);
            sftp.set_metadata(path.as_str(), attrs).await?;
        }

        Ok(file)
    }

    pub async fn list_dir(
        &self,
        path: String,
    ) -> Result<Vec<(String, FileAttributes)>, SshSessionError> {
        let sftp = self.sftp().await?;
        let read_dir = sftp.read_dir(path.as_str()).await?;
        Ok(read_dir.map(|entry| (entry.file_name(), entry.metadata())).collect())
    }

    pub async fn remove_file(&self, path: String) -> Result<(), SshSessionError> {
        let sftp = self.sftp().await?;
        sftp.remove_file(path.as_str()).await?;
        Ok(())
    }

    pub async fn remove_dir(&self, path: String) -> Result<(), SshSessionError> {
        let sftp = self.sftp().await?;
        sftp.remove_dir(path.as_str()).await?;
        Ok(())
    }

    pub async fn rename(&self, from: String, to: String) -> Result<(), SshSessionError> {
        let sftp = self.sftp().await?;
        sftp.rename(from, to).await?;
        Ok(())
    }

    pub async fn metadata(&self, path: String) -> Result<FileAttributes, SshSessionError> {
        let sftp = self.sftp().await?;
        let meta = sftp.metadata(path.as_str()).await?;
        Ok(meta)
    }

    pub async fn channel_direct_tcp_ip(
        &self,
        host: String,
        port: u16,
    ) -> Result<SshAsyncChannel, SshSessionError> {
        let channel = self
            .ssh_session
            .channel_open_direct_tcpip(host, port as u32, "127.0.0.1".to_string(), 0)
            .await?;
        Ok(channel.into_stream())
    }

    pub async fn channel_direct_streamlocal(
        &self,
        socket_path: String,
    ) -> Result<SshAsyncChannel, SshSessionError> {
        let channel = self
            .ssh_session
            .channel_open_direct_streamlocal(socket_path)
            .await?;
        Ok(channel.into_stream())
    }

    pub async fn execute_command(
        &self,
        command: &str,
    ) -> Result<(Vec<u8>, Vec<u8>, i32), SshSessionError> {
        let mut channel = self.ssh_session.channel_open_session().await?;
        channel.exec(true, command).await?;

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let mut exit_status: i32 = 0;

        while let Some(msg) = channel.wait().await {
            match msg {
                ChannelMsg::Data { ref data } => stdout.extend_from_slice(data),
                ChannelMsg::ExtendedData { ref data, ext } => {
                    if ext == 1 {
                        stderr.extend_from_slice(data);
                    } else {
                        stdout.extend_from_slice(data);
                    }
                }
                ChannelMsg::ExitStatus { exit_status: code } => exit_status = code as i32,
                ChannelMsg::Close => break,
                _ => {}
            }
        }

        Ok((stdout, stderr, exit_status))
    }

    pub async fn start_command(&self, command: &str) -> Result<RemoteProcess, SshSessionError> {
        let channel = self.ssh_session.channel_open_session().await?;
        channel.exec(true, command).await?;
        Ok(RemoteProcess::start(channel))
    }

    pub async fn disconnect(&self, description: String) {
        let _ = self
            .ssh_session
            .disconnect(russh::Disconnect::ByApplication, &description, "")
            .await;
    }
}

