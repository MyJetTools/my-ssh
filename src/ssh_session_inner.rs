use std::sync::Arc;

use async_ssh2_lite::{AsyncSession, AsyncSessionStream};

use crate::{SshAsyncSession, SshCredentials, SshSession, SshSessionError, SshSessionWrapper};

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

    pub async fn disconnect(&mut self, description: &str, host: &SshSession) {
        if let Some(session) = self.ssh_session.take() {
            session.disconnect(description).await;
        }

        host.connected.set_value(false);
    }
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
