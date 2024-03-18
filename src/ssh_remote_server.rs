use std::sync::Arc;

use crate::{SshAsyncChannel, SshCredentials, SshSessionError};

use super::SshRemoteConnection;

pub struct SshRemoteServer {
    remote_connections: Vec<Arc<SshRemoteConnection>>,
    ssh_credentials: Arc<SshCredentials>,
}

impl SshRemoteServer {
    pub fn new(ssh_credentials: SshCredentials) -> Self {
        Self {
            ssh_credentials: Arc::new(ssh_credentials),
            remote_connections: Vec::new(),
        }
    }

    pub fn add_remote_connection(
        mut self,
        listen_host_port: impl Into<String>,
        remote_host: impl Into<String>,
        remote_port: u16,
    ) -> Self {
        let new_item =
            SshRemoteConnection::new(listen_host_port.into(), remote_host.into(), remote_port);
        self.remote_connections.push(Arc::new(new_item));
        self
    }

    pub async fn connect_to_remote_host(
        &self,
        host: impl Into<String>,
        port: u16,
        timeout: std::time::Duration,
    ) -> Result<SshAsyncChannel, SshSessionError> {
        let ssh_session = crate::SSH_SESSION_POOL
            .get_or_create_ssh_session(&self.ssh_credentials)
            .await;

        ssh_session
            .connect_to_remote_host(host.into().as_str(), port, timeout)
            .await
    }

    pub fn start(self) -> Self {
        for remote_connection in &self.remote_connections {
            super::tcp_server::start(remote_connection.clone(), self.ssh_credentials.clone());
        }

        self
    }
}
