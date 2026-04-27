#[derive(Debug, Clone)]
pub enum ForwardTarget {
    Tcp { host: String, port: u16 },
    Unix { socket_path: String },
}

impl ForwardTarget {
    pub fn describe(&self) -> String {
        match self {
            ForwardTarget::Tcp { host, port } => format!("{}:{}", host, port),
            ForwardTarget::Unix { socket_path } => socket_path.clone(),
        }
    }
}
