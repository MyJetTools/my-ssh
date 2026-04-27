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
mod remote_process;
pub use remote_process::*;

pub type SshAsyncSession = russh::client::Handle<MySshClientHandler>;
pub type SshAsyncChannel = russh::ChannelStream<russh::client::Msg>;
pub type RemoteFile = russh_sftp::client::fs::File;

pub extern crate russh;
pub extern crate russh_sftp;

mod port_forward;
pub use port_forward::*;
mod ssh_sessions_pool;
pub use ssh_sessions_pool::*;

pub mod ssh_settings;

lazy_static::lazy_static! {
    pub static ref SSH_SESSIONS_POOL: Arc<crate::SshSessionsPool> = Arc::new(crate::SshSessionsPool::new());
}
