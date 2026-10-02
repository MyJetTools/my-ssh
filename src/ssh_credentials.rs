use rust_extensions::{ShortString, SHORT_STRING_MAX_LEN};

#[derive(Debug, Clone)]
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
    pub fn try_from_str(src: &str, auth_type: SshAuthenticationType) -> Option<Self> {
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
            SshAuthenticationType::SshAgent => Self::SshAgent {
                ssh_remote_host: host.to_string(),
                ssh_remote_port: port,
                ssh_user_name: user_name.to_string(),
            },
            SshAuthenticationType::UserNameAndPassword(password) => Self::UserNameAndPassword {
                ssh_remote_host: host.to_string(),
                ssh_remote_port: port,
                ssh_user_name: user_name.to_string(),
                password,
            },
            SshAuthenticationType::PrivateKey {
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

    // A line longer than the 255 bytes a ShortString holds is cut down to them.
    pub fn to_string(&self) -> ShortString {
        let (host, port) = self.get_host_port();
        let line = format!("{}@{}:{}", self.get_user_name(), host, port);

        let mut len = line.len().min(SHORT_STRING_MAX_LEN);
        while !line.is_char_boundary(len) {
            len -= 1;
        }

        let mut result = ShortString::new_empty();
        result.try_push_str(&line[..len]);
        result
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

    pub fn into_with_private_key(
        &self,
        new_private_key: String,
        new_passphrase: Option<String>,
    ) -> Self {
        match self {
            SshCredentials::SshAgent {
                ssh_remote_host,
                ssh_remote_port,
                ssh_user_name,
            } => SshCredentials::PrivateKey {
                ssh_remote_host: ssh_remote_host.to_string(),
                ssh_remote_port: *ssh_remote_port,
                ssh_user_name: ssh_user_name.to_string(),
                private_key: new_private_key,
                passphrase: new_passphrase,
            },
            SshCredentials::UserNameAndPassword {
                ssh_remote_host,
                ssh_remote_port,
                ssh_user_name,
                password: _,
            } => SshCredentials::PrivateKey {
                ssh_remote_host: ssh_remote_host.to_string(),
                ssh_remote_port: *ssh_remote_port,
                ssh_user_name: ssh_user_name.to_string(),
                private_key: new_private_key,
                passphrase: new_passphrase,
            },
            SshCredentials::PrivateKey {
                ssh_remote_host,
                ssh_remote_port,
                ssh_user_name,
                private_key: _,
                passphrase: _,
            } => SshCredentials::PrivateKey {
                ssh_remote_host: ssh_remote_host.to_string(),
                ssh_remote_port: *ssh_remote_port,
                ssh_user_name: ssh_user_name.to_string(),
                private_key: new_private_key,
                passphrase: new_passphrase,
            },
        }
    }
}

#[derive(Debug, Clone)]
pub enum SshAuthenticationType {
    SshAgent,
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
            SshCredentials::try_from_str("user@host:22", crate::SshAuthenticationType::SshAgent)
                .unwrap();
        assert_eq!(ssh_credentials.to_string().as_str(), "user@host:22");
    }

    #[test]
    fn test_without_port() {
        let ssh_credentials =
            SshCredentials::try_from_str("user@host", crate::SshAuthenticationType::SshAgent)
                .unwrap();
        assert_eq!(ssh_credentials.to_string().as_str(), "user@host:22");
    }

    fn ssh_agent(user_name: &str, host: &str) -> SshCredentials {
        SshCredentials::SshAgent {
            ssh_remote_host: host.to_string(),
            ssh_remote_port: 22,
            ssh_user_name: user_name.to_string(),
        }
    }

    fn password(user_name: &str, host: &str) -> SshCredentials {
        SshCredentials::UserNameAndPassword {
            ssh_remote_host: host.to_string(),
            ssh_remote_port: 22,
            ssh_user_name: user_name.to_string(),
            password: "password".to_string(),
        }
    }

    fn private_key(user_name: &str, host: &str) -> SshCredentials {
        SshCredentials::PrivateKey {
            ssh_remote_host: host.to_string(),
            ssh_remote_port: 22,
            ssh_user_name: user_name.to_string(),
            private_key: "private key".to_string(),
            passphrase: None,
        }
    }

    // A ShortString holds 255 bytes at most, so a longer line is cut down to them.
    fn first_255_bytes(user_name: &str, host: &str) -> String {
        format!("{}@{}:22", user_name, host)[..255].to_string()
    }

    #[test]
    fn test_user_name_longer_than_255_bytes_with_ssh_agent() {
        let user_name = "u".repeat(300);
        assert_eq!(
            ssh_agent(&user_name, "host").to_string().as_str(),
            first_255_bytes(&user_name, "host")
        );
    }

    #[test]
    fn test_user_name_longer_than_255_bytes_with_password() {
        let user_name = "u".repeat(300);
        assert_eq!(
            password(&user_name, "host").to_string().as_str(),
            first_255_bytes(&user_name, "host")
        );
    }

    #[test]
    fn test_user_name_longer_than_255_bytes_with_private_key() {
        let user_name = "u".repeat(300);
        assert_eq!(
            private_key(&user_name, "host").to_string().as_str(),
            first_255_bytes(&user_name, "host")
        );
    }

    // The user name and the host fit into 255 bytes each, but not together.
    #[test]
    fn test_user_name_and_host_longer_than_255_bytes_with_ssh_agent() {
        let (user_name, host) = ("u".repeat(200), "h".repeat(100));
        assert_eq!(
            ssh_agent(&user_name, &host).to_string().as_str(),
            first_255_bytes(&user_name, &host)
        );
    }

    #[test]
    fn test_user_name_and_host_longer_than_255_bytes_with_password() {
        let (user_name, host) = ("u".repeat(200), "h".repeat(100));
        assert_eq!(
            password(&user_name, &host).to_string().as_str(),
            first_255_bytes(&user_name, &host)
        );
    }

    #[test]
    fn test_user_name_and_host_longer_than_255_bytes_with_private_key() {
        let (user_name, host) = ("u".repeat(200), "h".repeat(100));
        assert_eq!(
            private_key(&user_name, &host).to_string().as_str(),
            first_255_bytes(&user_name, &host)
        );
    }

    #[test]
    fn test_line_of_255_bytes_is_not_cut() {
        let user_name = "u".repeat(255 - "@host:22".len());
        assert_eq!(
            ssh_agent(&user_name, "host").to_string().as_str(),
            format!("{}@host:22", user_name)
        );
    }

    #[test]
    fn test_line_of_256_bytes_is_cut() {
        let user_name = "u".repeat(256 - "@host:22".len());
        assert_eq!(
            ssh_agent(&user_name, "host").to_string().as_str(),
            first_255_bytes(&user_name, "host")
        );
    }

    // 'é' takes two bytes, so byte 255 falls into the middle of a character.
    #[test]
    fn test_long_line_is_cut_at_a_char_boundary() {
        let user_name = "é".repeat(150);
        assert_eq!(
            ssh_agent(&user_name, "host").to_string().as_str(),
            "é".repeat(127)
        );
    }
}
