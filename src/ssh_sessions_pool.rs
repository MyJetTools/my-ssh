use std::sync::Arc;

use tokio::sync::Mutex;

use crate::*;

pub struct SshSessionsPool {
    sessions: Mutex<Vec<Arc<SshSession>>>,
}

impl SshSessionsPool {
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(Vec::new()),
        }
    }

    pub async fn get_or_create(&self, ssh_credentials: &Arc<SshCredentials>) -> Arc<SshSession> {
        let mut sessions = self.sessions.lock().await;
        let mut i = 0;
        while i < sessions.len() {
            if sessions[i].get_ssh_credentials().are_same(ssh_credentials) {
                if sessions[i].is_alive().await {
                    return sessions[i].clone();
                } else {
                    sessions.swap_remove(i);
                    continue;
                }
            }
            i += 1;
        }

        let session = Arc::new(SshSession::new(ssh_credentials.clone()));

        sessions.push(session.clone());

        session
    }

    pub async fn get(&self, ssh_credentials: &SshCredentials) -> Option<Arc<SshSession>> {
        let mut sessions = self.sessions.lock().await;
        let mut i = 0;
        while i < sessions.len() {
            if sessions[i].get_ssh_credentials().are_same(ssh_credentials) {
                if sessions[i].is_alive().await {
                    return Some(sessions[i].clone());
                } else {
                    sessions.swap_remove(i);
                    continue;
                }
            }
            i += 1;
        }
        None
    }

    pub async fn insert(&self, ssh_session: &Arc<SshSession>) {
        let mut sessions = self.sessions.lock().await;

        sessions.retain(|session| {
            !session
                .get_ssh_credentials()
                .are_same(ssh_session.get_ssh_credentials())
        });
        sessions.push(ssh_session.clone());

        println!("Inserted Session. Sessions in cache: {}", sessions.len());
    }

    pub async fn remove(&self, ssh_credentials: &Arc<SshCredentials>) {
        let mut sessions = self.sessions.lock().await;
        sessions.retain(|session| !session.get_ssh_credentials().are_same(ssh_credentials));

        println!("Removed Session. Sessions in cache: {}", sessions.len());
    }
}
