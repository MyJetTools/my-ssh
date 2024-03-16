use std::sync::Arc;

use tokio::sync::RwLock;

use crate::{SshCredentials, SshSession};

pub struct SshSessionsPool {
    sessions: RwLock<Vec<Arc<SshSession>>>,
}

impl SshSessionsPool {
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(Vec::new()),
        }
    }

    pub async fn get_or_create_ssh_session(
        &self,
        ssh_credentials: &Arc<SshCredentials>,
    ) -> Arc<SshSession> {
        {
            let read_access = self.sessions.read().await;
            for itm in read_access.iter() {
                if itm.get_ssh_credentials().are_same(ssh_credentials.as_ref()) {
                    return itm.clone();
                }
            }
        }

        let mut write_access = self.sessions.write().await;
        for itm in write_access.iter() {
            if itm.get_ssh_credentials().are_same(ssh_credentials.as_ref()) {
                return itm.clone();
            }
        }

        let ssh_session = SshSession::new(ssh_credentials.clone());
        let ssh_session = Arc::new(ssh_session);

        write_access.push(ssh_session.clone());

        ssh_session
    }
}
