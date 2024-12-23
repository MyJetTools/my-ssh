use std::sync::Arc;

use crate::SshCredentials;

#[derive(Debug, Clone)]
pub struct SshPrivateKey {
    pub content: String,
    pub pass_phrase: Option<String>,
}

#[async_trait::async_trait]
pub trait SshSecurityCredentialsResolver {
    async fn resolve_ssh_private_key(&self, ssh_line: &str) -> Option<SshPrivateKey>;
    async fn resolve_ssh_password(&self, ssh_line: &str) -> Option<String>;

    async fn update_credentials(
        &self,
        ssh_line: &str,
        ssh_credentials: Arc<SshCredentials>,
    ) -> Arc<SshCredentials> {
        if let Some(private_key) = self.resolve_ssh_private_key(ssh_line).await {
            return Arc::new(SshCredentials::PrivateKey {
                ssh_remote_host: ssh_credentials.get_host_port().0.to_string(),
                ssh_remote_port: ssh_credentials.get_host_port().1,
                ssh_user_name: ssh_credentials.get_user_name().to_string(),
                private_key: private_key.content,
                passphrase: private_key.pass_phrase,
            });
        }

        if let Some(password) = self.resolve_ssh_password(ssh_line).await {
            return Arc::new(SshCredentials::UserNameAndPassword {
                ssh_remote_host: ssh_credentials.get_host_port().0.to_string(),
                ssh_remote_port: ssh_credentials.get_host_port().1,
                ssh_user_name: ssh_credentials.get_user_name().to_string(),
                password,
            });
        }

        ssh_credentials
    }
}
