use std::sync::atomic::{AtomicBool, Ordering};

use tokio::sync::Mutex;

use super::ForwardTarget;

pub struct SshPortForwardTunnel {
    pub listen_string: String,
    pub target: ForwardTarget,
    pub working: AtomicBool,
    pub task: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl SshPortForwardTunnel {
    pub fn new(listen_string: String, target: ForwardTarget) -> Self {
        Self {
            listen_string,
            target,
            working: AtomicBool::new(true),
            task: Mutex::new(None),
        }
    }

    pub fn is_working(&self) -> bool {
        self.working.load(Ordering::Relaxed)
    }

    pub async fn stop(&self) {
        let was_working = self.working.swap(false, Ordering::Relaxed);
        if was_working {
            let read_access = self.task.lock().await;
            if let Some(task) = &*read_access {
                task.abort();
            }
        }
    }
}

impl Drop for SshPortForwardTunnel {
    fn drop(&mut self) {
        self.working.store(false, Ordering::Relaxed);
    }
}
