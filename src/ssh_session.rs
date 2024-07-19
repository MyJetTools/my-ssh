use std::{sync::Arc, time::Duration};

use futures::Future;
use rust_extensions::{date_time::DateTimeAsMicroseconds, UnsafeValue};
use tokio::sync::Mutex;

use crate::{SshAsyncChannel, SshCredentials, SshSessionInner, SshSessionWrapper};

use super::SshSessionError;

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
        host: &str,
        port: u16,
        connection_timeout: Duration,
    ) -> Result<SshAsyncChannel, SshSessionError> {
        let mut write_access = self.inner.lock().await;
        let ssh_session = write_access.get(&self.credentials).await?;
        let future = ssh_session.channel_direct_tcp_ip(host, port);
        self.execute_with_timeout(&mut write_access, future, connection_timeout)
            .await
    }

    async fn get_home_variable(
        &self,
        ssh_session: &SshSessionWrapper,
        inner: &mut SshSessionInner,
        execute_timeout: Duration,
    ) -> Result<String, SshSessionError> {
        if inner.home_variable.is_none() {
            let home_variable = ssh_session.execute_command("echo $HOME");

            let (home_variable, _) = self
                .execute_with_timeout(inner, home_variable, execute_timeout)
                .await?;
            inner.home_variable = Some(home_variable.trim().to_string());
        }

        Ok(inner.home_variable.as_ref().unwrap().to_string())
    }

    pub async fn download_remote_file(
        &self,
        path: &str,
        execute_timeout: Duration,
    ) -> Result<Vec<u8>, SshSessionError> {
        let mut write_access = self.inner.lock().await;
        let ssh_session = write_access.get(&self.credentials).await?;

        let future = if path.starts_with("~") {
            let home_variable = self
                .get_home_variable(&ssh_session, &mut write_access, execute_timeout)
                .await?;

            let path = path.replace("~", home_variable.as_str());

            ssh_session.download_remote_file(path.into())
        } else {
            ssh_session.download_remote_file(path.into())
        };

        self.execute_with_timeout(&mut write_access, future, execute_timeout)
            .await
    }

    pub async fn upload_file(
        &self,
        remote_path: &str,
        content: &[u8],
        mode: i32,
        execute_timeout: Duration,
    ) -> Result<i32, SshSessionError> {
        let mut write_access = self.inner.lock().await;
        let ssh_session = write_access.get(&self.credentials).await?;

        let future = if remote_path.starts_with("~") {
            let home_variable = self
                .get_home_variable(&ssh_session, &mut write_access, execute_timeout)
                .await?;

            let remote_path = remote_path.replace("~", home_variable.as_str());

            ssh_session.upload_file(remote_path, content, mode)
        } else {
            ssh_session.upload_file(remote_path.to_string(), content, mode)
        };

        self.execute_with_timeout(&mut write_access, future, Duration::from_secs(10))
            .await
    }

    pub async fn execute_command(
        &self,
        command: &str,
        execute_timeout: Duration,
    ) -> Result<(String, i32), SshSessionError> {
        let mut write_access = self.inner.lock().await;
        let ssh_session = write_access.get(&self.credentials).await?;
        let future = ssh_session.execute_command(command);
        self.execute_with_timeout(&mut write_access, future, execute_timeout)
            .await
    }

    pub async fn set_port_forward(
        &self,
        listen_host: &str,
        remote_host: &str,
        remote_port: u16,
    ) -> Result<(), SshSessionError> {
        let mut write_access = self.inner.lock().await;
        let ssh_session = write_access.get(&self.credentials).await.unwrap();
        ssh_session
            .set_port_forward(remote_port, remote_host, listen_host)
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
