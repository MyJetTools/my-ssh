use std::sync::Arc;

use rust_extensions::str_utils::StrUtils;

// To help parsing connection settings from string like "ssh://user:password@host:port->http://localhost:8080"
pub struct OverSshConnectionSettings {
    pub ssh_credentials: Option<Arc<crate::SshCredentials>>,
    pub remote_resource_string: String,
}

impl OverSshConnectionSettings {
    pub fn parse(src: &str) -> Self {
        if !rust_extensions::str_utils::starts_with_case_insensitive(src, "ssh") {
            return Self {
                ssh_credentials: None,
                remote_resource_string: src.to_string(),
            };
        }

        let parsed = src.split_up_to_2_lines("->");

        if parsed.is_none() {
            panic!("Invalid resource to connect string: {}", src);
        }

        let (left_part, right_part) = parsed.unwrap();

        if right_part.is_none() {
            return Self {
                ssh_credentials: None,
                remote_resource_string: left_part.to_string(),
            };
        }

        let right_part = right_part.unwrap();

        Self {
            ssh_credentials: Some(Arc::new(parse_ssh_string(left_part))),
            remote_resource_string: right_part.to_string(),
        }
    }
}

// parsing line such as "ssh://username@host:port" or "ssh:username@host:port"
fn parse_ssh_string(src: &str) -> crate::SshCredentials {
    let split = src.split_exact_to_3_lines(":");

    if split.is_none() {
        panic!("Invalid ssh connection string: {}. Connection string must be like ssh://root@10.0.0.1:22", src);
    }

    let (_, user_name_and_host, port) = split.unwrap();

    let user_name_parsed = user_name_and_host.split_exact_to_2_lines("@");

    if user_name_parsed.is_none() {
        panic!(
            "Invalid user@host part '{}' in ssh connection string: {}.",
            user_name_and_host, src
        );
    }

    let (mut user_name, host) = user_name_parsed.unwrap();

    if user_name.starts_with("//") {
        user_name = &user_name[2..];
    }

    crate::SshCredentials::SshAgent {
        ssh_remote_host: host.to_string(),
        ssh_remote_port: port.parse().unwrap(),
        ssh_user_name: user_name.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::OverSshConnectionSettings;

    #[test]
    fn test() {
        let settings = "ssh://root@localhost:22->http://localhost:8080";

        let settings = OverSshConnectionSettings::parse(settings);

        assert_eq!("http://localhost:8080", settings.remote_resource_string);

        let settings = settings.ssh_credentials.unwrap();

        assert_eq!("root", settings.get_user_name());
        let (host, port) = settings.get_host_port();
        assert_eq!("localhost", host);
        assert_eq!(port, 22);
    }
}
