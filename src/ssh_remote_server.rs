use std::sync::Arc;

use crate::{SshAsyncChannel, SshCredentials, SshRemoteConnection, SshRemoteHost, SshSessionError};

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
        remote_host: &SshRemoteHost,
    ) -> Self {
        let new_item = SshRemoteConnection::new(listen_host_port.into(), remote_host.clone());
        self.remote_connections.push(Arc::new(new_item));
        self
    }

    pub async fn connect_to_remote_host(
        &self,
        host: &SshRemoteHost,
        timeout: std::time::Duration,
    ) -> Result<SshAsyncChannel, SshSessionError> {
        let ssh_session = crate::SSH_SESSION_POOL
            .get_or_create_ssh_session(&self.ssh_credentials)
            .await;

        ssh_session.connect_to_remote_host(host, timeout).await
    }

    pub fn start(self) -> Self {
        for remote_connection in &self.remote_connections {
            super::tcp_server::start(remote_connection.clone(), self.ssh_credentials.clone());
        }

        self
    }
}
