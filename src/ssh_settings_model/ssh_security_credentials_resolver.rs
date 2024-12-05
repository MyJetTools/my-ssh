#[derive(Debug, Clone)]
pub struct SshPrivateKey {
    pub content: String,
    pub pass_phrase: Option<String>,
}

#[async_trait::async_trait]
pub trait SshSecurityCredentialsResolver {
    async fn resolve_ssh_private_key(&self, ssh_line: &str) -> Option<SshPrivateKey>;
    async fn resolve_ssh_password(&self, ssh_line: &str) -> Option<String>;
}
