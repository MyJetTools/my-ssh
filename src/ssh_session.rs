use std::{sync::Arc, time::Duration};

use futures::Future;
use rust_extensions::{date_time::DateTimeAsMicroseconds, UnsafeValue};
use tokio::sync::Mutex;

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
    inner: Mutex<SshSessionInner>,
    credentials: Arc<SshCredentials>,
    pub id: i64,
    pub connected: UnsafeValue<bool>,
}

impl SshSession {
    pub fn new(credentials: Arc<SshCredentials>) -> Self {
        let id = DateTimeAsMicroseconds::now().unix_microseconds;
        Self {
            inner: Mutex::new(SshSessionInner::new()),
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
        let mut write_access = self.inner.lock().await;
        let ssh_session = write_access.get(&self.credentials).await?;
        let future = ssh_session.channel_direct_tcp_ip(remote_host);
        self.execute_with_timeout(&mut write_access, future, connection_timeout)
            .await
    }

    pub async fn download_remote_file(
        &self,
        path: &str,
        execute_timeout: Duration,
    ) -> Result<Vec<u8>, SshSessionError> {
        let mut write_access = self.inner.lock().await;
        let ssh_session = write_access.get(&self.credentials).await?;

        let future = if path.starts_with("~") {
            if write_access.home_variable.is_none() {
                let home_variable = ssh_session.execute_command("echo $HOME");

                let home_variable = self
                    .execute_with_timeout(&mut write_access, home_variable, execute_timeout)
                    .await?;
                write_access.home_variable = Some(home_variable.trim().to_string());
            }

            let path = path.replace("~", write_access.home_variable.as_ref().unwrap());

            ssh_session.download_remote_file(path.into())
        } else {
            ssh_session.download_remote_file(path.into())
        };

        self.execute_with_timeout(&mut write_access, future, execute_timeout)
            .await
    }

    pub async fn execute_command(
        &self,
        command: &str,
        execute_timeout: Duration,
    ) -> Result<String, SshSessionError> {
        let mut write_access = self.inner.lock().await;
        let ssh_session = write_access.get(&self.credentials).await?;
        let future = ssh_session.execute_command(command);
        self.execute_with_timeout(&mut write_access, future, execute_timeout)
            .await
    }

    pub async fn disconnect(&self, reason: &str) {
        let mut write_access = self.inner.lock().await;
        write_access.disconnect(reason, self).await;
    }

    pub fn is_connected(&self) -> bool {
        self.connected.get_value()
    }

    async fn execute_with_timeout<TResult>(
        &self,
        inner: &mut SshSessionInner,
        future: impl Future<Output = Result<TResult, SshSessionError>>,
        connection_timeout: Duration,
    ) -> Result<TResult, SshSessionError> {
        let result = tokio::time::timeout(connection_timeout, future).await;

        if result.is_err() {
            inner
                .disconnect("Timeout connecting to remote host", self)
                .await;
            return Err(SshSessionError::Timeout);
        }

        match result.unwrap() {
            Ok(result) => return Ok(result),
            Err(e) => {
                inner
                    .disconnect("Could not connect to remote host", self)
                    .await;
                return Err(e);
            }
        }
    }
}
