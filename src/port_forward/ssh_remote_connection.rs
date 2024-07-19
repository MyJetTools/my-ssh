use std::sync::atomic::{AtomicBool, Ordering};

pub struct SshPortForwardConnection {
    pub listen_host_port: String,
    pub remote_host: String,
    pub remote_port: u16,
    pub working: AtomicBool,
    pub stopped: AtomicBool,
}

impl SshPortForwardConnection {
    pub fn new(listen_host_port: String, remote_host: String, remote_port: u16) -> Self {
        Self {
            listen_host_port,
            remote_host,
            remote_port,
            working: AtomicBool::new(true),

            stopped: AtomicBool::new(false),
        }
    }

    pub fn is_working(&self) -> bool {
        self.working.load(Ordering::Relaxed)
    }

    pub fn mark_as_stopped(&self) {
        self.stopped.store(true, Ordering::Relaxed);
    }

    pub async fn stop(&self) {
        self.working.store(false, Ordering::Relaxed);
        loop {
            if self.stopped.load(Ordering::Relaxed) {
                break;
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }
}

impl Drop for SshPortForwardConnection {
    fn drop(&mut self) {
        self.working.store(false, Ordering::Relaxed);
    }
}
