#[derive(Debug)]
pub enum SshSessionError {
    SshSessionIsNotActive,
    StdIoStreamError(std::io::Error),
    SshError(async_ssh2_lite::Error),
    SshAuthenticationError,
    Timeout,
}

impl From<async_ssh2_lite::Error> for SshSessionError {
    fn from(error: async_ssh2_lite::Error) -> Self {
        SshSessionError::SshError(error)
    }
}

impl From<std::io::Error> for SshSessionError {
    fn from(error: std::io::Error) -> Self {
        SshSessionError::StdIoStreamError(error)
    }
}
