use std::sync::Arc;

use super::SshSession;

pub struct SshRemoteConnection {
    pub ssh_session: Arc<SshSession>,
    pub listen_host_port: String,
    pub remote_host: String,
    pub remote_port: u16,
}

impl SshRemoteConnection {
    pub fn new(
        ssh_session: Arc<SshSession>,
        listen_host_port: String,
        remote_host: String,
        remote_port: u16,
    ) -> Self {
        Self {
            listen_host_port,
            ssh_session,
            remote_host,
            remote_port,
        }
    }
}
