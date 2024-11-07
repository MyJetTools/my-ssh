use std::{net::SocketAddr, sync::Arc};

use async_ssh2_lite::{AsyncSession, SessionConfiguration};

use crate::{SshAsyncSession, SshCredentials, SshSessionError, SshSessionWrapper};

pub struct SshSessionInner {
    pub ssh_session: Option<Arc<SshSessionWrapper>>,
    pub home_variable: Option<String>,
}

impl SshSessionInner {
    pub fn new() -> Self {
        Self {
            ssh_session: None,
            home_variable: None,
        }
    }

    pub async fn get(
        &mut self,
        credentials: &Arc<SshCredentials>,
    ) -> Result<Arc<SshSessionWrapper>, SshSessionError> {
        if self.ssh_session.is_none() {
            let session = init_ssh_session(credentials).await?;
            self.ssh_session = Some(SshSessionWrapper::new(session).into());
        }

        Ok(self.ssh_session.as_ref().unwrap().clone())
    }

    pub async fn disconnect(&mut self, description: &str) {
        if let Some(session) = self.ssh_session.take() {
            session.disconnect(description).await;
        }
    }
}

pub async fn init_ssh_session(
    ssh_credentials: &Arc<SshCredentials>,
) -> Result<SshAsyncSession, SshSessionError> {
    let session = match ssh_credentials.as_ref() {
        SshCredentials::SshAgent {
            ssh_remote_host,
            ssh_remote_port,
            ssh_user_name,
        } => {
            let mut session_configuration = SessionConfiguration::new();
            session_configuration.set_compress(true);
            let mut session = AsyncSession::<async_ssh2_lite::TokioTcpStream>::connect(
                SocketAddr::new(ssh_remote_host.parse().unwrap(), *ssh_remote_port),
                Some(session_configuration),
            )
            .await?;

            session.handshake().await?;

            session.userauth_agent_with_try_next(ssh_user_name).await?;
            assert!(session.authenticated());

            session
        }
        SshCredentials::UserNameAndPassword {
            ssh_remote_host,
            ssh_remote_port,
            ssh_user_name,
            password,
        } => {
            let mut session = AsyncSession::<async_ssh2_lite::TokioTcpStream>::connect(
                SocketAddr::new(ssh_remote_host.parse().unwrap(), *ssh_remote_port),
                None,
            )
            .await?;

            session.handshake().await?;
            session.userauth_password(ssh_user_name, password).await?;

            assert!(session.authenticated());
            session
        }
        SshCredentials::PrivateKey {
            ssh_remote_host,
            ssh_remote_port,
            ssh_user_name,
            private_key,
            passphrase,
        } => {
            let mut session = AsyncSession::<async_ssh2_lite::TokioTcpStream>::connect(
                SocketAddr::new(ssh_remote_host.parse().unwrap(), *ssh_remote_port),
                None,
            )
            .await?;

            session.handshake().await?;

            let pass_phrase = if let Some(passphrase) = passphrase {
                Some(passphrase.as_str())
            } else {
                None
            };

            session
                .userauth_pubkey_memory(&ssh_user_name, None, private_key, pass_phrase)
                .await?;

            assert!(session.authenticated());
            session
        }
    };

    Ok(session)
}
