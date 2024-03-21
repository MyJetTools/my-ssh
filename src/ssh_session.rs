use std::{path::Path, sync::Arc, time::Duration};

use async_ssh2_lite::{AsyncSession, AsyncSessionStream};
use futures::AsyncReadExt;
use rust_extensions::{date_time::DateTimeAsMicroseconds, UnsafeValue};

use tokio::sync::Mutex;

use crate::{SshAsyncChannel, SshAsyncSession, SshCredentials};

use super::SshSessionError;

#[derive(Debug, Clone)]
pub struct SshRemoteHost {
    pub host: String,
    pub port: u16,
}

impl SshRemoteHost {
    pub fn to_socket_addr(&self) -> std::net::SocketAddr {
        std::net::SocketAddr::new(self.host.as_str().parse().unwrap(), self.port)
    }
}

impl SshRemoteHost {
    pub fn are_same(&self, other: &SshRemoteHost) -> bool {
        self.host == other.host && self.port == other.port
    }
}

pub struct SshSession {
    ssh_session: Mutex<Option<SshAsyncSession>>,
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
        remote_host: &SshRemoteHost,
        connection_timeout: Duration,
    ) -> Result<SshAsyncChannel, SshSessionError> {
        let mut session_access = self.ssh_session.lock().await;

        if session_access.is_none() {
            let session = init_ssh_session(self.get_ssh_credentials()).await?;
            *session_access = Some(session);
        }

        let ssh_session = session_access.as_ref().unwrap();

        let ssh_channel =
            ssh_session.channel_direct_tcpip(&remote_host.host, remote_host.port, None);

        let result = tokio::time::timeout(connection_timeout, ssh_channel).await;

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
        remote_host: &SshRemoteHost,
        connection_timeout: Duration,
    ) -> Result<SshAsyncChannel, SshSessionError> {
        let result = self
            .try_to_connect_to_remote_host(remote_host, connection_timeout)
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

    pub async fn download_remote_file(&self, path: &str) -> Result<Vec<u8>, SshSessionError> {
        let mut session_access = self.ssh_session.lock().await;

        if session_access.is_none() {
            let session = init_ssh_session(self.get_ssh_credentials()).await?;
            *session_access = Some(session);
        }

        let ssh_session = session_access.as_ref().unwrap();

        let (mut remote_file, _) = ssh_session.scp_recv(Path::new(path)).await?;

        let mut contents = Vec::new();
        remote_file.read_to_end(&mut contents).await?;
        remote_file.send_eof().await?;
        remote_file.wait_eof().await?;
        remote_file.close().await?;
        remote_file.wait_close().await?;
        Ok(contents)
    }

    pub fn is_connected(&self) -> bool {
        self.connected.get_value()
    }
}

fn execute_disconnect(session: &mut Option<SshAsyncSession>, connected: &UnsafeValue<bool>) {
    *session = None;
    connected.set_value(false);
}

pub async fn init_ssh_session(
    ssh_credentials: &Arc<SshCredentials>,
) -> Result<SshAsyncSession, SshSessionError> {
    let mut session = AsyncSession::<async_ssh2_lite::TokioTcpStream>::connect(
        ssh_credentials.get_host_port().to_socket_addr(),
        None,
    )
    .await?;

    run_session_user_auth_agent_with_try_next(&mut session, ssh_credentials.get_user_name())
        .await?;

    Ok(session)
}

async fn run_session_user_auth_agent_with_try_next<
    S: AsyncSessionStream + Send + Sync + 'static,
>(
    session: &mut AsyncSession<S>,
    user_name: &str,
) -> Result<(), SshSessionError> {
    session.handshake().await?;

    match session.userauth_agent_with_try_next(user_name).await {
        Ok(_) => {
            assert!(session.authenticated());
        }
        Err(err) => {
            eprintln!("session.userauth_agent_with_try_next failed, err:{err}");
            assert!(!session.authenticated());
        }
    }

    Ok(())
}

/*
async fn run_session_user_auth_agent<S: AsyncSessionStream + Send + Sync + 'static>(
    session: &mut AsyncSession<S>,
    user_name: &str,
) -> Result<(), SshSessionError> {
    session.handshake().await?;

    match session.userauth_agent(user_name).await {
        Ok(_) => {
            assert!(session.authenticated());
        }
        Err(err) => {
            eprintln!("session.userauth_agent failed, err:{err}");
            assert!(!session.authenticated());
        }
    }

    Ok(())
}
 */
