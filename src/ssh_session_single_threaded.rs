use std::sync::Arc;
use std::time::Duration;

use russh::client;
use russh::keys::agent::client::AgentClient;
use russh::keys::{HashAlg, PrivateKey, PrivateKeyWithHashAlg};

use crate::{SshAsyncSession, SshCredentials, SshSessionError, SshSessionWrapper};

pub struct MySshClientHandler;

impl client::Handler for MySshClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &russh::keys::ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

pub struct SshSessionSingleThreaded {
    pub ssh_session: Option<Arc<SshSessionWrapper>>,
    pub home_variable: Option<String>,
}

impl SshSessionSingleThreaded {
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
            let creds = credentials.clone();
            // Run the russh init in its own task to avoid leaking russh's
            // internal HRTB future state into the calling spawned future.
            let session = tokio::spawn(init_ssh_session_owned(creds))
                .await
                .map_err(|e| SshSessionError::Other(format!("join error: {:?}", e)))??;
            self.ssh_session = Some(SshSessionWrapper::new(session).into());
        }

        Ok(self.ssh_session.as_ref().unwrap().clone())
    }

    pub async fn disconnect(&mut self, description: String) {
        if let Some(session) = self.ssh_session.take() {
            // Run russh's disconnect in its own task so its future state
            // (which holds russh-internal references) does not propagate
            // into the caller's spawned future.
            let _ = tokio::spawn(async move {
                session.disconnect(description).await;
            })
            .await;
        }
    }
}

pub async fn init_ssh_session(
    ssh_credentials: &Arc<SshCredentials>,
) -> Result<SshAsyncSession, SshSessionError> {
    init_ssh_session_owned(ssh_credentials.clone()).await
}

pub async fn init_ssh_session_owned(
    ssh_credentials: Arc<SshCredentials>,
) -> Result<SshAsyncSession, SshSessionError> {
    let mut config = client::Config::default();
    config.keepalive_interval = Some(Duration::from_secs(30));
    config.keepalive_max = 3;
    let config = Arc::new(config);

    // Convert all credential fields into owned values up-front so that no
    // `&String` (or other borrowed) references from the `match` destructure
    // are kept alive across any russh await — borrows of locals would
    // otherwise propagate HRTB lifetimes that confuse Send auto-derivation.
    let prepared = match ssh_credentials.as_ref() {
        SshCredentials::SshAgent {
            ssh_remote_host,
            ssh_remote_port,
            ssh_user_name,
        } => PreparedAuth::Agent {
            addr: (ssh_remote_host.clone(), *ssh_remote_port),
            user_name: ssh_user_name.clone(),
        },
        SshCredentials::UserNameAndPassword {
            ssh_remote_host,
            ssh_remote_port,
            ssh_user_name,
            password,
        } => PreparedAuth::Password {
            addr: (ssh_remote_host.clone(), *ssh_remote_port),
            user_name: ssh_user_name.clone(),
            password: password.clone(),
        },
        SshCredentials::PrivateKey {
            ssh_remote_host,
            ssh_remote_port,
            ssh_user_name,
            private_key,
            passphrase,
        } => PreparedAuth::PrivateKey {
            addr: (ssh_remote_host.clone(), *ssh_remote_port),
            user_name: ssh_user_name.clone(),
            private_key: private_key.clone(),
            passphrase: passphrase.clone(),
        },
    };

    match prepared {
        PreparedAuth::Agent { addr, user_name } => {
            ssh_agent_authenticate(config, addr, user_name).await
        }
        PreparedAuth::Password {
            addr,
            user_name,
            password,
        } => password_authenticate(config, addr, user_name, password).await,
        PreparedAuth::PrivateKey {
            addr,
            user_name,
            private_key,
            passphrase,
        } => private_key_authenticate(config, addr, user_name, private_key, passphrase).await,
    }
}

enum PreparedAuth {
    Agent {
        addr: (String, u16),
        user_name: String,
    },
    Password {
        addr: (String, u16),
        user_name: String,
        password: String,
    },
    PrivateKey {
        addr: (String, u16),
        user_name: String,
        private_key: String,
        passphrase: Option<String>,
    },
}

async fn password_authenticate(
    config: Arc<client::Config>,
    addr: (String, u16),
    user_name: String,
    password: String,
) -> Result<SshAsyncSession, SshSessionError> {
    let mut session = client::connect(config, addr, MySshClientHandler).await?;
    let auth_res = session.authenticate_password(user_name, password).await?;
    if !auth_res.success() {
        return Err(SshSessionError::SshAuthenticationError);
    }
    Ok(session)
}

async fn private_key_authenticate(
    config: Arc<client::Config>,
    addr: (String, u16),
    user_name: String,
    private_key: String,
    passphrase: Option<String>,
) -> Result<SshAsyncSession, SshSessionError> {
    let mut session = client::connect(config, addr, MySshClientHandler).await?;

    let key =
        PrivateKey::from_openssh(private_key.as_bytes()).map_err(russh::keys::Error::from)?;

    let key = if let Some(ref passphrase) = passphrase {
        if key.is_encrypted() {
            key.decrypt(passphrase).map_err(russh::keys::Error::from)?
        } else {
            key
        }
    } else {
        key
    };

    let hash_alg: Option<HashAlg> = session.best_supported_rsa_hash().await?.flatten();
    let key_with_alg = PrivateKeyWithHashAlg::new(Arc::new(key), hash_alg);

    let auth_res = session
        .authenticate_publickey(user_name, key_with_alg)
        .await?;

    if !auth_res.success() {
        return Err(SshSessionError::SshAuthenticationError);
    }

    Ok(session)
}

async fn ssh_agent_authenticate(
    config: Arc<client::Config>,
    addr: (String, u16),
    user_name: String,
) -> Result<SshAsyncSession, SshSessionError> {
    let mut session = client::connect(config, addr, MySshClientHandler).await?;

    let mut agent = AgentClient::connect_env()
        .await
        .map_err(SshSessionError::SshKeysError)?;
    let identities = agent
        .request_identities()
        .await
        .map_err(SshSessionError::SshKeysError)?;

    if identities.is_empty() {
        return Err(SshSessionError::Other(
            "ssh-agent has no identities".to_string(),
        ));
    }

    let hash_alg = session.best_supported_rsa_hash().await?.flatten();
    let mut authenticated = false;
    for identity in identities {
        let public_key = identity.public_key().into_owned();
        let auth_res = session
            .authenticate_publickey_with(user_name.clone(), public_key, hash_alg, &mut agent)
            .await
            .map_err(|e| SshSessionError::Other(format!("agent auth error: {:?}", e)))?;

        if auth_res.success() {
            authenticated = true;
            break;
        }
    }

    if !authenticated {
        return Err(SshSessionError::SshAuthenticationError);
    }

    Ok(session)
}
