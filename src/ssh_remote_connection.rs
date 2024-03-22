pub struct SshRemoteConnection {
    pub listen_host_port: String,
    pub remote_host: String,
    pub remote_port: u16,
}

impl SshRemoteConnection {
    pub fn new(listen_host_port: String, remote_host: String, remote_port: u16) -> Self {
        Self {
            listen_host_port,
            remote_host,
            remote_port,
        }
    }
}
