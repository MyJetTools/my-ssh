use std::{sync::Arc, time::Duration};

use futures::Future;
use rust_extensions::{date_time::DateTimeAsMicroseconds, StrOrString, UnsafeValue};
use tokio::sync::Mutex;

use crate::{
    RemotePortForwardError, SshAsyncChannel, SshCredentials, SshPortForwardTunnel,
    SshSessionSingleThreaded, SshSessionWrapper,
};

use super::SshSessionError;

pub struct SshSessionInnerL {
    inner: Arc<Mutex<SshSessionSingleThreaded>>,
    pub credentials: Arc<SshCredentials>,
    pub id: i64,
    pub connected: UnsafeValue<bool>,
}

impl SshSessionInnerL {
    pub fn new(credentials: Arc<SshCredentials>) -> Self {
        let id = DateTimeAsMicroseconds::now().unix_microseconds;

        let using = match credentials.as_ref() {
            SshCredentials::SshAgent {
                ssh_remote_host,
                ssh_remote_port,
                ssh_user_name,
            } => "using ssh agent",
            SshCredentials::UserNameAndPassword {
                ssh_remote_host,
                ssh_remote_port,
                ssh_user_name,
                password,
            } => "using username and password",
            SshCredentials::PrivateKey {
                ssh_remote_host,
                ssh_remote_port,
                ssh_user_name,
                private_key,
                passphrase,
            } => {
                if passphrase.is_some() {
                    "using private key protected with passphrase"
                } else {
                    "using private key not protected with no passphrase"
                }
            }
        };

        println!(
            "Created ssh connection {} [{}]. {}",
            using,
            credentials.to_string(),
            id
        );
        Self {
            inner: Arc::new(Mutex::new(SshSessionSingleThreaded::new())),
            credentials,
            id,
            connected: UnsafeValue::new(true),
        }
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

    async fn execute_with_timeout<TResult>(
        &self,
        inner: &mut SshSessionSingleThreaded,
        future: impl Future<Output = Result<TResult, SshSessionError>>,
        connection_timeout: Duration,
    ) -> Result<TResult, SshSessionError> {
        let result = tokio::time::timeout(connection_timeout, future).await;

        if result.is_err() {
            inner.disconnect("Timeout connecting to remote host").await;
            self.connected.set_value(false);
            return Err(SshSessionError::Timeout);
        }

        match result.unwrap() {
            Ok(result) => return Ok(result),
            Err(e) => {
                inner.disconnect("Could not connect to remote host").await;
                self.connected.set_value(false);
                return Err(e);
            }
        }
    }

    pub async fn disconnect(&self, reason: &str) {
        let mut write_access = self.inner.lock().await;
        write_access.disconnect(reason).await;
        self.connected.set_value(false);
    }
}

pub struct SshSession {
    pub inner: Arc<SshSessionInnerL>,
}

impl SshSession {
    pub fn new(credentials: Arc<SshCredentials>) -> Self {
        Self {
            inner: Arc::new(SshSessionInnerL::new(credentials)),
        }
    }

    pub fn get_ssh_credentials(&self) -> &Arc<SshCredentials> {
        &self.inner.credentials
    }

    pub async fn connect_to_remote_host(
        &self,
        host: &str,
        port: u16,
        connection_timeout: Duration,
    ) -> Result<SshAsyncChannel, SshSessionError> {
        self.inner
            .connect_to_remote_host(host, port, connection_timeout)
            .await
    }

    async fn get_home_variable(
        &self,
        ssh_session: &SshSessionWrapper,
        inner: &mut SshSessionSingleThreaded,
        execute_timeout: Duration,
    ) -> Result<String, SshSessionError> {
        if inner.home_variable.is_none() {
            let home_variable = ssh_session.execute_command("echo $HOME");

            let (home_variable, _) = self
                .inner
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
        let mut write_access = self.inner.inner.lock().await;
        let ssh_session = write_access.get(&self.inner.credentials).await?;

        let future = if path.starts_with("~") {
            let home_variable = self
                .get_home_variable(&ssh_session, &mut write_access, execute_timeout)
                .await?;

            let path = path.replace("~", home_variable.as_str());

            ssh_session.download_remote_file(path.into())
        } else {
            ssh_session.download_remote_file(path.into())
        };

        self.inner
            .execute_with_timeout(&mut write_access, future, execute_timeout)
            .await
    }

    pub async fn upload_file(
        &self,
        remote_path: &str,
        content: &[u8],
        mode: i32,
        execute_timeout: Duration,
    ) -> Result<i32, SshSessionError> {
        let mut write_access = self.inner.inner.lock().await;
        let ssh_session = write_access.get(&self.inner.credentials).await?;

        let future = if remote_path.starts_with("~") {
            let home_variable = self
                .get_home_variable(&ssh_session, &mut write_access, execute_timeout)
                .await?;

            let remote_path = remote_path.replace("~", home_variable.as_str());

            ssh_session.upload_file(remote_path, content, mode)
        } else {
            ssh_session.upload_file(remote_path.to_string(), content, mode)
        };

        self.inner
            .execute_with_timeout(&mut write_access, future, Duration::from_secs(10))
            .await
    }

    pub async fn execute_command(
        &self,
        command: &str,
        execute_timeout: Duration,
    ) -> Result<(String, i32), SshSessionError> {
        let mut write_access = self.inner.inner.lock().await;
        let ssh_session = write_access.get(&self.inner.credentials).await?;
        let future = ssh_session.execute_command(command);
        self.inner
            .execute_with_timeout(&mut write_access, future, execute_timeout)
            .await
    }

    pub async fn disconnect(&self, reason: &str) {
        self.inner.disconnect(reason).await;
    }

    pub fn is_connected(&self) -> bool {
        self.inner.connected.get_value()
    }

    pub async fn start_port_forward(
        &self,
        listen_host_port: impl Into<StrOrString<'static>>,
        remote_host: impl Into<StrOrString<'static>>,
        remote_port: u16,
    ) -> Result<Arc<SshPortForwardTunnel>, RemotePortForwardError> {
        let new_item = SshPortForwardTunnel::new(
            listen_host_port.into().to_string(),
            remote_host.into().to_string(),
            remote_port,
        );

        let new_item = Arc::new(new_item);

        crate::port_forward::start(new_item.clone(), self.inner.clone()).await?;

        Ok(new_item)
    }
}

impl Drop for SshSession {
    fn drop(&mut self) {
        let inner = self.inner.clone();

        println!(
            "Dropping Ssh Session [{}]. {}",
            self.inner.credentials.to_string(),
            self.inner.id
        );

        tokio::spawn(async move {
            let mut inner_access = inner.inner.lock().await;
            inner_access.disconnect("Shutting down").await;
        });
    }
}
