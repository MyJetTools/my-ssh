#[derive(Debug)]
pub enum SshCredentials {
    SshAgent {
        ssh_remote_host: String,
        ssh_remote_port: u16,
        ssh_user_name: String,
    },
    UserNameAndPassword {
        ssh_remote_host: String,
        ssh_remote_port: u16,
        ssh_user_name: String,
        password: String,
    },

    PrivateKey {
        ssh_remote_host: String,
        ssh_remote_port: u16,
        ssh_user_name: String,
        private_key: String,
        passphrase: Option<String>,
    },
}

impl SshCredentials {
    pub fn try_from_str(src: &str, auth_type: AuthenticationType) -> Option<Self> {
        let mut parts = src.split('@');

        let user_name = parts.next()?;

        let mut parts = parts.next()?.split(':');

        let host = parts.next()?;

        let port = if let Some(port) = parts.next() {
            let port = port.parse::<u16>().ok()?;
            port
        } else {
            22
        };

        let result = match auth_type {
            AuthenticationType::UserAgent => Self::SshAgent {
                ssh_remote_host: host.to_string(),
                ssh_remote_port: port,
                ssh_user_name: user_name.to_string(),
            },
            AuthenticationType::UserNameAndPassword(password) => Self::UserNameAndPassword {
                ssh_remote_host: host.to_string(),
                ssh_remote_port: port,
                ssh_user_name: user_name.to_string(),
                password,
            },
            AuthenticationType::PrivateKey {
                private_key_content,
                pass_phrase,
            } => Self::PrivateKey {
                ssh_remote_host: host.to_string(),
                ssh_remote_port: port,
                ssh_user_name: user_name.to_string(),
                private_key: private_key_content,
                passphrase: pass_phrase,
            },
        };

        Some(result)
    }

    pub fn to_string(&self) -> String {
        match self {
            SshCredentials::SshAgent {
                ssh_remote_host,
                ssh_remote_port,
                ssh_user_name,
            } => format!("{}@{}:{}", ssh_user_name, ssh_remote_host, ssh_remote_port),
            SshCredentials::UserNameAndPassword {
                ssh_remote_host,
                ssh_remote_port,
                ssh_user_name,
                ..
            } => format!("{}@{}:{}", ssh_user_name, ssh_remote_host, ssh_remote_port),
            SshCredentials::PrivateKey {
                ssh_remote_host,
                ssh_remote_port,
                ssh_user_name,
                ..
            } => format!("{}@{}:{}", ssh_user_name, ssh_remote_host, ssh_remote_port),
        }
    }
    pub fn are_same(&self, other: &SshCredentials) -> bool {
        match self {
            SshCredentials::SshAgent {
                ssh_remote_host,
                ssh_remote_port,
                ssh_user_name,
            } => match other {
                SshCredentials::SshAgent {
                    ssh_remote_host: other_ssh_remote_host,
                    ssh_remote_port: other_ssh_remote_port,
                    ssh_user_name: other_user_name,
                } => {
                    ssh_remote_host == other_ssh_remote_host
                        && ssh_remote_port == other_ssh_remote_port
                        && ssh_user_name == other_user_name
                }
                SshCredentials::UserNameAndPassword { .. } => false,
                SshCredentials::PrivateKey { .. } => false,
            },
            SshCredentials::UserNameAndPassword {
                ssh_remote_host,
                ssh_remote_port,
                ssh_user_name,
                password,
            } => match other {
                SshCredentials::SshAgent { .. } => false,
                SshCredentials::PrivateKey { .. } => false,
                SshCredentials::UserNameAndPassword {
                    ssh_remote_host: other_ssh_remote_host,
                    ssh_remote_port: other_ssh_remote_port,
                    ssh_user_name: other_user_name,
                    password: other_password,
                } => {
                    ssh_remote_host == other_ssh_remote_host
                        && ssh_remote_port == other_ssh_remote_port
                        && ssh_user_name == other_user_name
                        && password == other_password
                }
            },
            SshCredentials::PrivateKey {
                ssh_remote_host,
                ssh_remote_port,
                ssh_user_name,
                private_key,
                passphrase,
            } => match other {
                SshCredentials::SshAgent { .. } => false,
                SshCredentials::UserNameAndPassword { .. } => false,
                SshCredentials::PrivateKey {
                    ssh_remote_host: other_ssh_remote_host,
                    ssh_remote_port: other_ssh_remote_port,
                    ssh_user_name: other_user_name,
                    private_key: other_private_key,
                    passphrase: other_passphrase,
                } => {
                    ssh_remote_host == other_ssh_remote_host
                        && ssh_remote_port == other_ssh_remote_port
                        && ssh_user_name == other_user_name
                        && passphrase == other_passphrase
                        && private_key == other_private_key
                }
            },
        }
    }

    pub fn get_host_port(&self) -> (&str, u16) {
        match self {
            SshCredentials::SshAgent {
                ssh_remote_host,
                ssh_remote_port,
                ..
            } => (ssh_remote_host.as_str(), *ssh_remote_port),
            SshCredentials::UserNameAndPassword {
                ssh_remote_host,
                ssh_remote_port,
                ..
            } => (ssh_remote_host.as_str(), *ssh_remote_port),
            SshCredentials::PrivateKey {
                ssh_remote_host,
                ssh_remote_port,
                ..
            } => (ssh_remote_host.as_str(), *ssh_remote_port),
        }
    }

    pub fn get_host_port_as_string(&self) -> String {
        match self {
            SshCredentials::SshAgent {
                ssh_remote_host,
                ssh_remote_port,
                ..
            } => format!("{}:{}", ssh_remote_host, ssh_remote_port),
            SshCredentials::UserNameAndPassword {
                ssh_remote_host,
                ssh_remote_port,
                ..
            } => format!("{}:{}", ssh_remote_host, ssh_remote_port),
            SshCredentials::PrivateKey {
                ssh_remote_host,
                ssh_remote_port,
                ..
            } => format!("{}:{}", ssh_remote_host, ssh_remote_port),
        }
    }

    pub fn get_user_name(&self) -> &str {
        match self {
            SshCredentials::SshAgent { ssh_user_name, .. } => ssh_user_name.as_str(),
            SshCredentials::UserNameAndPassword { ssh_user_name, .. } => ssh_user_name.as_str(),
            SshCredentials::PrivateKey { ssh_user_name, .. } => ssh_user_name.as_str(),
        }
    }
}

pub enum AuthenticationType {
    UserAgent,
    UserNameAndPassword(String),
    PrivateKey {
        private_key_content: String,
        pass_phrase: Option<String>,
    },
}

#[cfg(test)]
mod tests {
    use crate::SshCredentials;

    #[test]
    fn test_with_port() {
        let ssh_credentials =
            SshCredentials::try_from_str("user@host:22", crate::AuthenticationType::UserAgent)
                .unwrap();
        assert_eq!(ssh_credentials.to_string(), "user@host:22");
    }

    #[test]
    fn test_without_port() {
        let ssh_credentials =
            SshCredentials::try_from_str("user@host", crate::AuthenticationType::UserAgent)
                .unwrap();
        assert_eq!(ssh_credentials.to_string(), "user@host:22");
    }
}
