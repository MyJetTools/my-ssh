mod error;
use std::sync::Arc;

pub use error::*;
mod ssh_session;
pub use ssh_session::*;

mod ssh_credentials;
pub use ssh_credentials::*;
mod ssh_session_single_threaded;
pub use ssh_session_single_threaded::*;
mod ssh_session_wrapper;
pub use ssh_session_wrapper::*;

pub type SshAsyncSession = async_ssh2_lite::AsyncSession<async_ssh2_lite::TokioTcpStream>;

pub type SshAsyncChannel = async_ssh2_lite::AsyncChannel<async_ssh2_lite::TokioTcpStream>;

pub extern crate ssh2;
mod port_forward;
pub use port_forward::*;
mod ssh_sessions_pool;
pub use ssh_sessions_pool::*;

pub mod ssh_settings;

lazy_static::lazy_static! {
    pub static ref SSH_SESSIONS_POOL: Arc<crate::SshSessionsPool> =  Arc::new(crate::SshSessionsPool::new());
}
