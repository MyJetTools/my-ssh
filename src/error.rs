#[derive(Debug)]
pub enum SshSessionError {
    SshSessionIsNotActive,
    StdIoStreamError(std::io::Error),
    SshError(russh::Error),
    SshKeysError(russh::keys::Error),
    SftpError(russh_sftp::client::error::Error),
    SshAuthenticationError,
    Other(String),
    Timeout,
}

impl From<russh::Error> for SshSessionError {
    fn from(error: russh::Error) -> Self {
        SshSessionError::SshError(error)
    }
}

impl From<russh::keys::Error> for SshSessionError {
    fn from(error: russh::keys::Error) -> Self {
        SshSessionError::SshKeysError(error)
    }
}

impl From<russh_sftp::client::error::Error> for SshSessionError {
    fn from(error: russh_sftp::client::error::Error) -> Self {
        SshSessionError::SftpError(error)
    }
}

impl From<std::io::Error> for SshSessionError {
    fn from(error: std::io::Error) -> Self {
        SshSessionError::StdIoStreamError(error)
    }
}
