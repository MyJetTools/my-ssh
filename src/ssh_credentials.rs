pub enum SshCredentials {
    SshAgent {
        ssh_host_port: String,
        ssh_user_name: String,
    },
}

impl SshCredentials {
    pub fn are_same(&self, other: &SshCredentials) -> bool {
        match self {
            SshCredentials::SshAgent {
                ssh_host_port,
                ssh_user_name,
            } => match other {
                SshCredentials::SshAgent {
                    ssh_host_port: other_ssh_host_port,
                    ssh_user_name: other_user_name,
                } => {
                    rust_extensions::str_utils::compare_strings_case_insensitive(
                        ssh_host_port,
                        other_ssh_host_port,
                    ) && ssh_user_name == other_user_name
                }
            },
        }
    }

    pub fn get_host_port(&self) -> &str {
        match self {
            SshCredentials::SshAgent { ssh_host_port, .. } => ssh_host_port.as_str(),
        }
    }

    pub fn get_user_name(&self) -> &str {
        match self {
            SshCredentials::SshAgent { ssh_user_name, .. } => ssh_user_name.as_str(),
        }
    }
}
