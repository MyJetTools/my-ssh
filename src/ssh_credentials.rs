#[derive(Debug)]
pub enum SshCredentials {
    SshAgent {
        ssh_remote_host: String,
        ssh_remote_port: u16,
        ssh_user_name: String,
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
        }
    }

    pub fn get_host_port_as_string(&self) -> String {
        match self {
            SshCredentials::SshAgent {
                ssh_remote_host,
                ssh_remote_port,
                ..
            } => format!("{}:{}", ssh_remote_host, ssh_remote_port),
        }
    }

    pub fn get_user_name(&self) -> &str {
        match self {
            SshCredentials::SshAgent { ssh_user_name, .. } => ssh_user_name.as_str(),
        }
    }
}
