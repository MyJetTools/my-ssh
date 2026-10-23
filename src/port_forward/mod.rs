mod forward_target;
mod ssh_port_forward_tunnel;
mod ssh_port_forward_tunnels_pool;
mod tcp_server;
#[cfg(unix)]
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
        #[cfg(unix)]
        return unix_socket_server::start(tunnel, ssh_session).await;

        // A path to listen on is a unix socket, and there are none to bind on this platform.
        #[cfg(not(unix))]
        return Err(RemotePortForwardError::ErrorBindingUnixSocket(format!(
            "Unix sockets are not supported on this platform: {}",
            tunnel.listen_string
        )));
    } else {
        tcp_server::start(tunnel, ssh_session).await
    }
}
