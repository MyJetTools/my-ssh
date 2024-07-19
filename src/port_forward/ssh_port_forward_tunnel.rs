use std::sync::atomic::{AtomicBool, Ordering};

pub struct SshPortForwardTunnel {
    pub listen_string: String,
    pub remote_host: String,
    pub remote_port: u16,
    pub working: AtomicBool,
    pub stopped: AtomicBool,
}

impl SshPortForwardTunnel {
    pub fn new(listen_string: String, remote_host: String, remote_port: u16) -> Self {
        Self {
            listen_string,
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

impl Drop for SshPortForwardTunnel {
    fn drop(&mut self) {
        self.working.store(false, Ordering::Relaxed);
    }
}
