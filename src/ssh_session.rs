use std::{net::TcpStream, sync::Arc, time::Duration};

use rust_extensions::{date_time::DateTimeAsMicroseconds, UnsafeValue};
use ssh2::*;
use tokio::sync::Mutex;

use crate::SshCredentials;

use super::SshSessionError;

pub struct SshSession {
    ssh_session: Mutex<Option<Session>>,
    credentials: Arc<SshCredentials>,
    pub id: i64,
    connected: UnsafeValue<bool>,
}

impl SshSession {
    pub fn new(credentials: Arc<SshCredentials>) -> Self {
        Self {
            ssh_session: Mutex::new(None),
            credentials,
            id: DateTimeAsMicroseconds::now().unix_microseconds,
            connected: UnsafeValue::new(true),
        }
    }

    pub fn get_ssh_credentials(&self) -> &Arc<SshCredentials> {
        &self.credentials
    }

    async fn try_to_connect_to_remote_host(
        &self,
        remote_host: &str,
        remote_port: u16,
        connection_timeout: Duration,
    ) -> Result<ssh2::Channel, SshSessionError> {
        let mut session_access = self.ssh_session.lock().await;

        if session_access.is_none() {
            let session = init_ssh_session(self.get_ssh_credentials())?;
            *session_access = Some(session);
        }

        let ssh_session = session_access.as_ref().unwrap();

        let result = tokio::time::timeout(
            connection_timeout,
            crate::async_ssh_channel::connect(ssh_session, remote_host, remote_port),
        )
        .await;

        if result.is_err() {
            execute_disconnect(&mut session_access, &self.connected);
            return Err(SshSessionError::Timeout);
        }

        match result.unwrap() {
            Ok(channel) => return Ok(channel),
            Err(e) => {
                execute_disconnect(&mut session_access, &self.connected);
                return Err(SshSessionError::SshError(e));
            }
        }
    }

    pub async fn connect_to_remote_host(
        &self,
        remote_host: &str,
        remote_port: u16,
        connection_timeout: Duration,
    ) -> Result<ssh2::Channel, SshSessionError> {
        let result = self
            .try_to_connect_to_remote_host(remote_host, remote_port, connection_timeout)
            .await;

        if result.is_err() {
            crate::SSH_SESSION_POOL.remove_from_pool(self).await;
        }

        result
    }

    pub async fn disconnect(&self) {
        {
            let mut write_access = self.ssh_session.lock().await;
            execute_disconnect(&mut write_access, &self.connected);
        }

        crate::SSH_SESSION_POOL.remove_from_pool(self).await;
    }

    pub fn is_connected(&self) -> bool {
        self.connected.get_value()
    }
}

fn execute_disconnect(session: &mut Option<Session>, connected: &UnsafeValue<bool>) {
    *session = None;
    connected.set_value(false);
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
