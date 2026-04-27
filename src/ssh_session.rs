use std::{sync::Arc, time::Duration};

use rust_extensions::{date_time::DateTimeAsMicroseconds, StrOrString};
use russh_sftp::client::fs::File;
use russh_sftp::protocol::{FileAttributes, OpenFlags};
use tokio::sync::Mutex;

use crate::{
    ForwardTarget, RemotePortForwardError, RemoteProcess, SshAsyncChannel, SshCredentials,
    SshPortForwardTunnel, SshSessionSingleThreaded, SshSessionWrapper,
};

use super::SshSessionError;

pub struct SshSessionInnerL {
    inner: Arc<Mutex<SshSessionSingleThreaded>>,
    pub credentials: Arc<SshCredentials>,
    pub id: i64,
}

impl SshSessionInnerL {
    pub fn new(credentials: Arc<SshCredentials>) -> Self {
        let id = DateTimeAsMicroseconds::now().unix_microseconds;

        let using = match credentials.as_ref() {
            SshCredentials::SshAgent { .. } => "using ssh agent",
            SshCredentials::UserNameAndPassword { .. } => "using username and password",
            SshCredentials::PrivateKey { passphrase, .. } => {
                if passphrase.is_some() {
                    "using private key protected with passphrase"
                } else {
                    "using private key without passphrase"
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
        }
    }

    /// True if the session is either not yet opened (lazy) or its underlying
    /// russh handle is still alive. Returns false only when we have an opened
    /// handle that has been closed (e.g. by keepalive timeout).
    pub async fn is_alive(&self) -> bool {
        let guard = self.inner.lock().await;
        match guard.ssh_session.as_ref() {
            Some(wrapper) => !wrapper.is_closed(),
            None => true,
        }
    }

    pub async fn open_remote_tcp_stream(
        &self,
        host: String,
        port: u16,
        timeout: Duration,
    ) -> Result<SshAsyncChannel, SshSessionError> {
        let wrapper = self.acquire().await?;
        let task = tokio::spawn(async move {
            tokio::time::timeout(timeout, wrapper.channel_direct_tcp_ip(host, port)).await
        });
        unwrap_join_timeout(task.await)?
    }

    pub async fn open_remote_unix_stream(
        &self,
        socket_path: String,
        timeout: Duration,
    ) -> Result<SshAsyncChannel, SshSessionError> {
        let wrapper = self.acquire().await?;
        let task = tokio::spawn(async move {
            tokio::time::timeout(timeout, wrapper.channel_direct_streamlocal(socket_path)).await
        });
        unwrap_join_timeout(task.await)?
    }

    async fn acquire(&self) -> Result<Arc<SshSessionWrapper>, SshSessionError> {
        let mut guard = self.inner.lock().await;
        guard.get(&self.credentials).await
    }

    pub async fn disconnect(&self, reason: String) {
        let mut guard = self.inner.lock().await;
        guard.disconnect(reason).await;
    }

    /// Resolves a path that may start with `~` to an absolute path on the
    /// remote host, caching `$HOME` on first use.
    async fn resolve_path(
        &self,
        wrapper: &Arc<SshSessionWrapper>,
        path: &str,
        timeout: Duration,
    ) -> Result<String, SshSessionError> {
        if !path.starts_with('~') {
            return Ok(path.to_string());
        }

        let home = self.get_home_variable(wrapper, timeout).await?;
        Ok(path.replacen('~', home.as_str(), 1))
    }

    async fn get_home_variable(
        &self,
        wrapper: &Arc<SshSessionWrapper>,
        timeout: Duration,
    ) -> Result<String, SshSessionError> {
        {
            let guard = self.inner.lock().await;
            if let Some(home) = guard.home_variable.as_ref() {
                return Ok(home.clone());
            }
        }

        let wrapper_clone = wrapper.clone();
        let task = tokio::spawn(async move {
            tokio::time::timeout(timeout, wrapper_clone.execute_command("echo $HOME")).await
        });
        let (stdout, _stderr, _exit) = unwrap_join_timeout(task.await)??;

        let home = String::from_utf8_lossy(&stdout).trim().to_string();
        if home.is_empty() {
            return Err(SshSessionError::Other("could not resolve $HOME".to_string()));
        }

        let mut guard = self.inner.lock().await;
        guard.home_variable = Some(home.clone());

        Ok(home)
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

    pub async fn is_alive(&self) -> bool {
        self.inner.is_alive().await
    }

    /// Open a TCP-tunneled stream to `host:port` reachable from the SSH server.
    /// Returns a tokio `AsyncRead + AsyncWrite + Unpin + Send + 'static` stream.
    pub async fn open_remote_tcp_stream(
        &self,
        host: &str,
        port: u16,
        timeout: Duration,
    ) -> Result<SshAsyncChannel, SshSessionError> {
        self.inner
            .open_remote_tcp_stream(host.to_string(), port, timeout)
            .await
    }

    /// Open a Unix-socket-tunneled stream to a remote `socket_path` on the SSH
    /// server (uses OpenSSH `direct-streamlocal@openssh.com`).
    pub async fn open_remote_unix_stream(
        &self,
        socket_path: &str,
        timeout: Duration,
    ) -> Result<SshAsyncChannel, SshSessionError> {
        let resolved = self.expand_tilde(socket_path, timeout).await?;
        self.inner.open_remote_unix_stream(resolved, timeout).await
    }

    /// Create a remote directory (recursive, like `mkdir -p`). `~` is
    /// expanded to the SSH user's home directory on the remote side.
    pub async fn create_remote_dir(
        &self,
        path: &str,
        timeout: Duration,
    ) -> Result<(), SshSessionError> {
        let wrapper = self.inner.acquire().await?;
        let resolved = self.inner.resolve_path(&wrapper, path, timeout).await?;
        let task = tokio::spawn(async move {
            tokio::time::timeout(timeout, wrapper.mkdir_recursive(resolved)).await
        });
        unwrap_join_timeout(task.await)?
    }

    /// Open a remote file via SFTP. Returns a `RemoteFile` (alias for
    /// `russh_sftp::client::fs::File`) that implements tokio
    /// `AsyncRead + AsyncWrite + AsyncSeek`.
    ///
    /// `mode` is applied via SFTP `setstat` after the open and is only
    /// useful when creating a new file (`OpenFlags::CREATE`).
    pub async fn open_remote_file(
        &self,
        path: &str,
        flags: OpenFlags,
        mode: Option<u32>,
        timeout: Duration,
    ) -> Result<File, SshSessionError> {
        let wrapper = self.inner.acquire().await?;
        let resolved = self.inner.resolve_path(&wrapper, path, timeout).await?;
        let task = tokio::spawn(async move {
            tokio::time::timeout(timeout, wrapper.open_file(resolved, flags, mode)).await
        });
        unwrap_join_timeout(task.await)?
    }

    /// List entries of a remote directory via SFTP. Returns
    /// `(filename, FileAttributes)` pairs (skips `.` and `..`).
    pub async fn list_remote_dir(
        &self,
        path: &str,
        timeout: Duration,
    ) -> Result<Vec<(String, FileAttributes)>, SshSessionError> {
        let wrapper = self.inner.acquire().await?;
        let resolved = self.inner.resolve_path(&wrapper, path, timeout).await?;
        let task = tokio::spawn(async move {
            tokio::time::timeout(timeout, wrapper.list_dir(resolved)).await
        });
        unwrap_join_timeout(task.await)?
    }

    pub async fn remove_remote_file(
        &self,
        path: &str,
        timeout: Duration,
    ) -> Result<(), SshSessionError> {
        let wrapper = self.inner.acquire().await?;
        let resolved = self.inner.resolve_path(&wrapper, path, timeout).await?;
        let task = tokio::spawn(async move {
            tokio::time::timeout(timeout, wrapper.remove_file(resolved)).await
        });
        unwrap_join_timeout(task.await)?
    }

    pub async fn remove_remote_dir(
        &self,
        path: &str,
        timeout: Duration,
    ) -> Result<(), SshSessionError> {
        let wrapper = self.inner.acquire().await?;
        let resolved = self.inner.resolve_path(&wrapper, path, timeout).await?;
        let task = tokio::spawn(async move {
            tokio::time::timeout(timeout, wrapper.remove_dir(resolved)).await
        });
        unwrap_join_timeout(task.await)?
    }

    pub async fn rename_remote(
        &self,
        from: &str,
        to: &str,
        timeout: Duration,
    ) -> Result<(), SshSessionError> {
        let wrapper = self.inner.acquire().await?;
        let from_resolved = self.inner.resolve_path(&wrapper, from, timeout).await?;
        let to_resolved = self.inner.resolve_path(&wrapper, to, timeout).await?;
        let task = tokio::spawn(async move {
            tokio::time::timeout(timeout, wrapper.rename(from_resolved, to_resolved)).await
        });
        unwrap_join_timeout(task.await)?
    }

    pub async fn remote_metadata(
        &self,
        path: &str,
        timeout: Duration,
    ) -> Result<FileAttributes, SshSessionError> {
        let wrapper = self.inner.acquire().await?;
        let resolved = self.inner.resolve_path(&wrapper, path, timeout).await?;
        let task = tokio::spawn(async move {
            tokio::time::timeout(timeout, wrapper.metadata(resolved)).await
        });
        unwrap_join_timeout(task.await)?
    }

    /// One-shot exec: returns `(stdout, stderr, exit_code)`.
    pub async fn execute_command(
        &self,
        command: &str,
        timeout: Duration,
    ) -> Result<(Vec<u8>, Vec<u8>, i32), SshSessionError> {
        let wrapper = self.inner.acquire().await?;
        let cmd = command.to_string();
        let task = tokio::spawn(async move {
            tokio::time::timeout(timeout, async move {
                wrapper.execute_command(&cmd).await
            })
            .await
        });
        unwrap_join_timeout(task.await)?
    }

    /// Start an interactive remote process. The returned `RemoteProcess`
    /// exposes stdout/stderr as `AsyncRead`, stdin as `AsyncWrite`, plus
    /// `signal()` and `wait_exit()`.
    pub async fn start_command(&self, command: &str) -> Result<RemoteProcess, SshSessionError> {
        let wrapper = self.inner.acquire().await?;
        wrapper.start_command(command).await
    }

    pub async fn disconnect(&self, reason: &str) {
        self.inner.disconnect(reason.to_string()).await;
    }

    /// Forward a local listener (`"host:port"` or `/path/to/sock`) to a TCP
    /// target reachable from the SSH server.
    pub async fn start_port_forward_to_tcp(
        &self,
        listen_host_port: impl Into<StrOrString<'static>>,
        remote_host: impl Into<String>,
        remote_port: u16,
    ) -> Result<Arc<SshPortForwardTunnel>, RemotePortForwardError> {
        self.start_port_forward(
            listen_host_port,
            ForwardTarget::Tcp {
                host: remote_host.into(),
                port: remote_port,
            },
        )
        .await
    }

    /// Forward a local listener to a Unix socket on the SSH server (uses
    /// `direct-streamlocal@openssh.com`).
    pub async fn start_port_forward_to_unix(
        &self,
        listen_host_port: impl Into<StrOrString<'static>>,
        remote_socket_path: impl Into<String>,
    ) -> Result<Arc<SshPortForwardTunnel>, RemotePortForwardError> {
        self.start_port_forward(
            listen_host_port,
            ForwardTarget::Unix {
                socket_path: remote_socket_path.into(),
            },
        )
        .await
    }

    async fn start_port_forward(
        &self,
        listen_host_port: impl Into<StrOrString<'static>>,
        target: ForwardTarget,
    ) -> Result<Arc<SshPortForwardTunnel>, RemotePortForwardError> {
        let new_item = SshPortForwardTunnel::new(listen_host_port.into().to_string(), target);
        let new_item = Arc::new(new_item);

        crate::port_forward::start(new_item.clone(), self.inner.clone()).await?;

        Ok(new_item)
    }

    async fn expand_tilde(&self, path: &str, timeout: Duration) -> Result<String, SshSessionError> {
        if !path.starts_with('~') {
            return Ok(path.to_string());
        }
        let wrapper = self.inner.acquire().await?;
        self.inner.resolve_path(&wrapper, path, timeout).await
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
            inner_access.disconnect("Shutting down".to_string()).await;
        });
    }
}

fn unwrap_join_timeout<T>(
    join_result: Result<Result<T, tokio::time::error::Elapsed>, tokio::task::JoinError>,
) -> Result<T, SshSessionError> {
    match join_result {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(_elapsed)) => Err(SshSessionError::Timeout),
        Err(e) => Err(SshSessionError::Other(format!("join error: {:?}", e))),
    }
}
