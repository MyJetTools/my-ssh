use std::collections::HashMap;

use rust_extensions::{remote_endpoint::RemoteEndpoint, str_utils::StrUtils};
use serde::*;
use tokio::sync::Mutex;

lazy_static::lazy_static! {
    pub static ref SSH_CREDENTIALS: Mutex<HashMap<String, String>> = Mutex::new(HashMap::new()) ;
}

// To help parsing connection settings from string like "ssh://user:password@host:port->http://localhost:8080"
pub struct OverSshConnectionSettings {
    pub ssh_credentials: Option<crate::SshCredentials>,
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
            ssh_credentials: Some(parse_ssh_string(left_part)),
            remote_resource_string: right_part.to_string(),
        }
    }

    pub fn get_remote_endpoint<'s>(&'s self) -> RemoteEndpoint<'s> {
        RemoteEndpoint::try_parse(&self.remote_resource_string).unwrap()
    }
}

// parsing line such as "ssh://username@host:port" or "ssh:username@host:port"
fn parse_ssh_string(src: &str) -> crate::SshCredentials {
    let split = src.split_2_or_3_lines(":");

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

    let port = if let Some(port) = port {
        port.parse().unwrap()
    } else {
        22
    };

    crate::SshCredentials::SshAgent {
        ssh_remote_host: host.to_string(),
        ssh_remote_port: port,
        ssh_user_name: user_name.to_string(),
    }
}

pub async fn load_cert(data: &SshCredentialsSettingsModel, src: &str) -> String {
    let mut ssh_credentials = SSH_CREDENTIALS.lock().await;
    if let Some(cert_content) = ssh_credentials.get(src) {
        return cert_content.to_string();
    }
    let file = rust_extensions::file_utils::format_path(data.cert_path.as_str());
    let cert_content = tokio::fs::read_to_string(file.as_str()).await;

    if let Err(err) = &cert_content {
        panic!(
            "Error reading certificate file: {}. Err: {:?}",
            file.as_str(),
            err
        );
    }

    let cert_content = cert_content.unwrap();

    ssh_credentials.insert(src.to_string(), cert_content.to_string());

    cert_content
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SshCredentialsSettingsModel {
    pub cert_path: String,
    pub cert_pass_prase: String,
}

#[cfg(test)]
mod tests {
    use super::OverSshConnectionSettings;

    #[test]
    fn test() {
        let settings = "ssh://root@localhost:222->http://localhost:8080";

        let settings = OverSshConnectionSettings::parse(settings);

        assert_eq!("http://localhost:8080", settings.remote_resource_string);

        let settings = settings.ssh_credentials.unwrap();

        assert_eq!("root", settings.get_user_name());
        let (host, port) = settings.get_host_port();
        assert_eq!("localhost", host);
        assert_eq!(port, 222);
    }

    #[test]
    fn test_without_port_at_ssh() {
        let settings = "ssh://root@localhost->http://localhost:8080";

        let settings = OverSshConnectionSettings::parse(settings);

        assert_eq!("http://localhost:8080", settings.remote_resource_string);

        let settings = settings.ssh_credentials.unwrap();

        assert_eq!("root", settings.get_user_name());
        let (host, port) = settings.get_host_port();
        assert_eq!("localhost", host);
        assert_eq!(port, 22);
    }
}
