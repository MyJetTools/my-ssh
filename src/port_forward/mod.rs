mod forward_target;
mod ssh_port_forward_tunnel;
mod ssh_port_forward_tunnels_pool;
mod tcp_server;
mod unix_socket_server;

pub use forward_target::*;
pub use ssh_port_forward_tunnel::*;
pub use ssh_port_forward_tunnels_pool::*;

use std::sync::Arc;

use crate::SshSessionInnerL;

pub async fn start(
    tunnel: Arc<SshPortForwardTunnel>,
    ssh_session: Arc<SshSessionInnerL>,
) -> Result<(), RemotePortForwardError> {
    if tunnel.listen_string.starts_with('/') {
        unix_socket_server::start(tunnel, ssh_session).await
    } else {
        tcp_server::start(tunnel, ssh_session).await
    }
}
