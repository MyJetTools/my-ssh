use std::sync::Arc;

use super::{SshRemoteConnection, SshSession};

pub struct SshPortMapServer {
    remote_connections: Vec<Arc<SshRemoteConnection>>,
    ssh_session: Arc<SshSession>,
}

impl SshPortMapServer {
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

    pub fn start(self) -> Self {
        for remote_connection in &self.remote_connections {
            super::tcp_server::start(remote_connection.clone());
        }

        self
    }
}
