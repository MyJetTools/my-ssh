use std::net::TcpStream;

use ssh2::*;
use tokio::sync::Mutex;

use super::SshSessionError;

pub struct SshSession {
    ssh_session: Mutex<Option<Session>>,
    ssh_host_port: String,
    user_name: String,
}

impl SshSession {
    pub fn new(ssh_host_port: String, user_name: String) -> Self {
        Self {
            ssh_session: Mutex::new(None),
            ssh_host_port,
            user_name,
        }
    }

    pub async fn connect_to_remote_host(
        &self,
        remote_host: &str,
        remote_port: u16,
    ) -> Result<ssh2::Channel, SshSessionError> {
        let mut session_access = self.ssh_session.lock().await;

        if session_access.is_none() {
            let session = init_ssh_session(self.ssh_host_port.as_str(), self.user_name.as_str())?;
            *session_access = Some(session);
        }

        let ssh_session = session_access.as_ref().unwrap();

        let result = crate::async_ssh_channel::connect(ssh_session, remote_host, remote_port).await;

        match result {
            Ok(channel) => Ok(channel),
            Err(e) => {
                *session_access = None;
                return Err(SshSessionError::SshError(e));
            }
        }
    }

    pub async fn disconnect(&self) {
        let mut write_access = self.ssh_session.lock().await;
        *write_access = None;
    }
}

pub fn init_ssh_session(ssh_host_port: &str, user_name: &str) -> Result<Session, SshSessionError> {
    let tcp = TcpStream::connect(ssh_host_port)?;
    println!("Connected to {}", ssh_host_port);
    let mut ssh_session = Session::new()?;

    ssh_session.set_tcp_stream(tcp);
    ssh_session.handshake()?;

    // Try to authenticate with the first identity in the agent.

    ssh_session.userauth_agent(user_name)?;

    // Make sure we succeeded
    if !ssh_session.authenticated() {
        return Err(SshSessionError::SshAuthenticationError);
    }

    ssh_session.set_blocking(false);

    Ok(ssh_session)
}
