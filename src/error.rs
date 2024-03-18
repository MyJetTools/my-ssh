#[derive(Debug)]
pub enum SshSessionError {
    SshSessionIsNotActive,
    StdIoStreamError(std::io::Error),
    SshError(ssh2::Error),
    SshAuthenticationError,
    Timeout,
}

impl From<ssh2::Error> for SshSessionError {
    fn from(error: ssh2::Error) -> Self {
        SshSessionError::SshError(error)
    }
}

impl From<std::io::Error> for SshSessionError {
    fn from(error: std::io::Error) -> Self {
        SshSessionError::StdIoStreamError(error)
    }
}
