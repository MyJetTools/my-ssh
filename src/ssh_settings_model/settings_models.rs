use serde::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SshPrivateKeySettingsModel {
    pub cert_path: String,
    pub cert_pass_phrase: Option<String>,
}

impl SshPrivateKeySettingsModel {
    pub async fn load_cert(&self) -> String {
        let file = rust_extensions::file_utils::format_path(self.cert_path.as_str());
        let cert_content = tokio::fs::read_to_string(file.as_str()).await;

        if let Err(err) = &cert_content {
            panic!(
                "Error reading certificate file: {}. Err: {:?}",
                file.as_str(),
                err
            );
        }

        let cert_content = cert_content.unwrap();
        cert_content
    }
}
