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
