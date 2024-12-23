mod ssh_port_forward_tunnels_pool;

use std::sync::Arc;

pub use ssh_port_forward_tunnels_pool::*;
mod ssh_port_forward_tunnel;
mod tcp_server;
pub use ssh_port_forward_tunnel::*;

use crate::SshSessionInnerL;
mod unix_socket_server;

pub async fn start(
    remote_connection: Arc<SshPortForwardTunnel>,
    ssh_session: Arc<SshSessionInnerL>,
) -> Result<(), RemotePortForwardError> {
    if remote_connection.listen_string.starts_with('/') {
        unix_socket_server::start(remote_connection, ssh_session).await
    } else {
        tcp_server::start(remote_connection, ssh_session).await
    }
}
