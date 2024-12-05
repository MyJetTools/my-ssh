use std::{collections::BTreeMap, sync::Arc};

use rust_extensions::StrOrString;
use tokio::sync::Mutex;

use crate::{SshPortForwardTunnel, SshSessionInnerL};

#[derive(Debug)]
pub enum RemotePortForwardError {
    CanNotExtractListenPort(String),
    CanNotBindListenEndpoint(String),
    ErrorBindingUnixSocket(String),
}

pub struct SshPortForwardTunnelsPool {
    remote_connections: Mutex<BTreeMap<u16, Arc<SshPortForwardTunnel>>>,
    ssh_session: Arc<SshSessionInnerL>,
}

impl SshPortForwardTunnelsPool {
    pub fn new(ssh_session: Arc<SshSessionInnerL>) -> Self {
        Self {
            ssh_session,
            remote_connections: Mutex::new(BTreeMap::new()),
        }
    }

    pub async fn add_remote_connection(
        &self,
        listen_host_port: impl Into<StrOrString<'static>>,
        remote_host: impl Into<String>,
        remote_port: u16,
    ) -> Result<Option<Arc<SshPortForwardTunnel>>, RemotePortForwardError> {
        let listen_host_port: StrOrString = listen_host_port.into();
        let listen_port = extract_port(listen_host_port.as_str())?;

        let mut connections_access = self.remote_connections.lock().await;
        let new_item =
            SshPortForwardTunnel::new(listen_host_port.into(), remote_host.into(), remote_port);

        let new_item = Arc::new(new_item);

        super::start(new_item.clone(), self.ssh_session.clone()).await?;

        let old_item = connections_access.insert(listen_port, new_item);

        Ok(old_item)
    }

    pub async fn find_connection(
        &self,
        check: impl Fn(&SshPortForwardTunnel) -> bool,
    ) -> Option<Arc<SshPortForwardTunnel>> {
        let read_access = self.remote_connections.lock().await;
        for connection in read_access.values() {
            if check(connection.as_ref()) {
                return Some(connection.clone());
            }
        }

        None
    }

    pub async fn remove_connection(&self, port: u16) -> Option<Arc<SshPortForwardTunnel>> {
        let mut write_access = self.remote_connections.lock().await;
        write_access.remove(&port)
    }
}

fn extract_port(str: &str) -> Result<u16, RemotePortForwardError> {
    let value = str.split(":").last();

    if value.is_none() {
        return Err(RemotePortForwardError::CanNotExtractListenPort(format!(
            "There is no port in the string: {}",
            str
        )));
    }

    let value = value.unwrap();

    match value.parse() {
        Ok(port) => Ok(port),
        Err(err) => Err(RemotePortForwardError::CanNotExtractListenPort(format!(
            "Error parsing port from string: '{}'. Err: {}",
            str, err
        ))),
    }
}
