use std::{net::TcpStream, sync::Arc};

use ssh2::*;
use tokio::sync::Mutex;

use crate::SshCredentials;

use super::SshSessionError;

pub struct SshSession {
    ssh_session: Mutex<Option<Session>>,
    credentials: Arc<SshCredentials>,
}

impl SshSession {
    pub fn new(credentials: Arc<SshCredentials>) -> Self {
        Self {
            ssh_session: Mutex::new(None),
            credentials,
        }
    }

    pub fn get_ssh_credentials(&self) -> &Arc<SshCredentials> {
        &self.credentials
    }

    pub async fn connect_to_remote_host(
        &self,
        remote_host: &str,
        remote_port: u16,
    ) -> Result<ssh2::Channel, SshSessionError> {
        let mut session_access = self.ssh_session.lock().await;

        if session_access.is_none() {
            let session = init_ssh_session(self.get_ssh_credentials())?;
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

pub fn init_ssh_session(ssh_credentials: &Arc<SshCredentials>) -> Result<Session, SshSessionError> {
    let tcp = TcpStream::connect(ssh_credentials.get_host_port())?;
    println!("Connected to {}", ssh_credentials.get_user_name());
    let mut ssh_session = Session::new()?;

    ssh_session.set_tcp_stream(tcp);
    ssh_session.handshake()?;

    // Try to authenticate with the first identity in the agent.

    ssh_session.userauth_agent(ssh_credentials.get_user_name())?;

    // Make sure we succeeded
    if !ssh_session.authenticated() {
        return Err(SshSessionError::SshAuthenticationError);
    }

    ssh_session.set_blocking(false);

    Ok(ssh_session)
}
