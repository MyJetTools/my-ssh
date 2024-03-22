use std::{sync::Arc, time::Duration};

use futures::Future;
use rust_extensions::{date_time::DateTimeAsMicroseconds, UnsafeValue};

use crate::{SshAsyncChannel, SshCredentials, SshSessionInner};

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

    pub fn to_string(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    pub fn are_same(&self, other: &SshRemoteHost) -> bool {
        self.host == other.host && self.port == other.port
    }
}

pub struct SshSession {
    inner: SshSessionInner,
    credentials: Arc<SshCredentials>,
    pub id: i64,
    pub connected: UnsafeValue<bool>,
}

impl SshSession {
    pub fn new(credentials: Arc<SshCredentials>) -> Self {
        let id = DateTimeAsMicroseconds::now().unix_microseconds;
        Self {
            inner: SshSessionInner::new(),
            credentials,
            id,
            connected: UnsafeValue::new(true),
        }
    }

    pub fn get_ssh_credentials(&self) -> &Arc<SshCredentials> {
        &self.credentials
    }

    pub async fn connect_to_remote_host(
        &self,
        remote_host: &SshRemoteHost,
        connection_timeout: Duration,
    ) -> Result<SshAsyncChannel, SshSessionError> {
        let ssh_session = self.inner.get(&self.credentials).await?;
        let future = ssh_session.channel_direct_tcp_ip(remote_host);
        self.execute_with_timeout(future, connection_timeout).await
    }

    pub async fn download_remote_file(
        &self,
        path: &str,
        execute_timeout: Duration,
    ) -> Result<Vec<u8>, SshSessionError> {
        let ssh_session = self.inner.get(&self.credentials).await?;
        let future = ssh_session.download_remote_file(path);
        self.execute_with_timeout(future, execute_timeout).await
    }

    pub async fn execute_command(
        &self,
        command: &str,
        execute_timeout: Duration,
    ) -> Result<String, SshSessionError> {
        let ssh_session = self.inner.get(&self.credentials).await?;
        let future = ssh_session.execute_command(command);
        self.execute_with_timeout(future, execute_timeout).await
    }

    pub async fn disconnect(&self, reason: &str) {
        self.inner.disconnect(reason, self).await;
    }

    pub fn is_connected(&self) -> bool {
        self.connected.get_value()
    }

    async fn execute_with_timeout<TResult>(
        &self,
        future: impl Future<Output = Result<TResult, SshSessionError>>,
        connection_timeout: Duration,
    ) -> Result<TResult, SshSessionError> {
        let result = tokio::time::timeout(connection_timeout, future).await;

        if result.is_err() {
            self.inner
                .disconnect("Timeout connecting to remote host", self)
                .await;
            return Err(SshSessionError::Timeout);
        }

        match result.unwrap() {
            Ok(result) => return Ok(result),
            Err(e) => {
                self.inner
                    .disconnect("Could not connect to remote host", self)
                    .await;
                return Err(e);
            }
        }
    }
}
