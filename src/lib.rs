mod ssh_remote_connection;
mod tcp_server;
use std::sync::Arc;

pub use ssh_remote_connection::*;
mod error;
pub use error::*;
mod ssh_session;
pub use ssh_session::*;

mod ssh_remote_server;

pub use ssh_remote_server::*;
//pub mod async_ssh_channel;
mod ssh_sessions_pool;
pub use ssh_sessions_pool::*;

mod ssh_credentials;
pub use ssh_credentials::*;

pub type SshAsyncSession = async_ssh2_lite::AsyncSession<async_ssh2_lite::TokioTcpStream>;

pub type SshAsyncChannel = async_ssh2_lite::AsyncChannel<async_ssh2_lite::TokioTcpStream>;

lazy_static::lazy_static! {
    pub static ref SSH_SESSION_POOL: Arc<SshSessionsPool> = {
        Arc::new(SshSessionsPool::new())
    };
}
