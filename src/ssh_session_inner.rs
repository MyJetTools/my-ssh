use std::sync::Arc;

use async_ssh2_lite::{AsyncSession, AsyncSessionStream};
use tokio::sync::Mutex;

use crate::{SshAsyncSession, SshCredentials, SshSession, SshSessionError, SshSessionWrapper};

pub struct SshSessionInner {
    pub ssh_session: Mutex<Option<Arc<SshSessionWrapper>>>,
    pub home_variable: Option<String>,
}

impl SshSessionInner {
    pub fn new() -> Self {
        Self {
            ssh_session: Mutex::new(None),
            home_variable: None,
        }
    }

    pub async fn get(
        &self,
        credentials: &Arc<SshCredentials>,
    ) -> Result<Arc<SshSessionWrapper>, SshSessionError> {
        let mut write_access = self.ssh_session.lock().await;
        if write_access.is_none() {
            let session = init_ssh_session(credentials).await?;
            *write_access = Some(Arc::new(SshSessionWrapper::new(session)));
        }

        Ok(write_access.as_ref().unwrap().clone())
    }

    pub async fn disconnect(&self, description: &str, host: &SshSession) {
        {
            let mut write_access = self.ssh_session.lock().await;
            if let Some(session) = write_access.take() {
                session.disconnect(description).await;
            }
        }
        host.connected.set_value(false);
        crate::SSH_SESSION_POOL.remove_from_pool(host).await;
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
