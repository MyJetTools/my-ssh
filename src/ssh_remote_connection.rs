use crate::SshRemoteHost;

pub struct SshRemoteConnection {
    pub listen_host_port: String,
    pub remote_host: SshRemoteHost,
}

impl SshRemoteConnection {
    pub fn new(listen_host_port: String, remote_host: SshRemoteHost) -> Self {
        Self {
            listen_host_port,
            remote_host,
        }
    }
}
