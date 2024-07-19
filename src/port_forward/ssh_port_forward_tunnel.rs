use std::sync::atomic::{AtomicBool, Ordering};

use tokio::sync::Mutex;

pub struct SshPortForwardTunnel {
    pub listen_string: String,
    pub remote_host: String,
    pub remote_port: u16,
    pub working: AtomicBool,
    pub task: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl SshPortForwardTunnel {
    pub fn new(listen_string: String, remote_host: String, remote_port: u16) -> Self {
        Self {
            listen_string,
            remote_host,
            remote_port,
            working: AtomicBool::new(true),

            task: Mutex::new(None),
        }
    }

    pub fn is_working(&self) -> bool {
        self.working.load(Ordering::Relaxed)
    }

    pub async fn stop(&self) {
        {
            let read_access = self.task.lock().await;
            if let Some(task) = &*read_access {
                task.abort();
            }
        }
        self.working.store(false, Ordering::Relaxed);
    }
}

impl Drop for SshPortForwardTunnel {
    fn drop(&mut self) {
        self.working.store(false, Ordering::Relaxed);
    }
}
