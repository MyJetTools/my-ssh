use std::sync::Arc;

use crate::SshSessionError;

use super::{SshRemoteConnection, SshSession};

pub struct SshRemoteServer {
    remote_connections: Vec<Arc<SshRemoteConnection>>,
    ssh_session: Arc<SshSession>,
}

impl SshRemoteServer {
    pub fn new(ssh_host_port: impl Into<String>, ssh_user_name: impl Into<String>) -> Self {
        Self {
            ssh_session: Arc::new(SshSession::new(ssh_host_port.into(), ssh_user_name.into())),
            remote_connections: Vec::new(),
        }
    }

    pub fn add_remote_connection(
        mut self,
        listen_host_port: impl Into<String>,
        remote_host: impl Into<String>,
        remote_port: u16,
    ) -> Self {
        let new_item = SshRemoteConnection::new(
            self.ssh_session.clone(),
            listen_host_port.into(),
            remote_host.into(),
            remote_port,
        );
        self.remote_connections.push(Arc::new(new_item));
        self
    }

    pub async fn connect_to_remote_host(
        &self,
        host: impl Into<String>,
        port: u16,
    ) -> Result<ssh2::Channel, SshSessionError> {
        self.ssh_session
            .connect_to_remote_host(host.into().as_str(), port)
            .await
    }

    pub fn start(self) -> Self {
        for remote_connection in &self.remote_connections {
            super::tcp_server::start(remote_connection.clone());
        }

        self
    }
}
