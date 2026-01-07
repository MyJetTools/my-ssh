# my-ssh

Async SSH helpers built on `async-ssh2-lite` for running commands, transferring files, and creating local port-forwards (TCP sockets or Unix sockets) with minimal boilerplate.

## Features
- SSH authentication via agent, username/password, or in-memory private key
- Session pooling (`SSH_SESSIONS_POOL`) to reuse authenticated sessions
- Command execution with timeouts
- Download/upload files (supports `~` expansion)
- Local port-forwarding to a remote host (TCP listener or Unix socket listener)
- Helper for parsing `ssh://user@host:port->…` connection strings with pluggable secret resolvers

## Install

```toml
[dependencies]
my-ssh = { tag="${last_tag}" git = "git@github.com:MyJetTools/my-ssh.git" }
```

## Quickstart: run a command

```rust
use std::{sync::Arc, time::Duration};
use my_ssh::{SshAuthenticationType, SshCredentials, SSH_SESSIONS_POOL};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Build credentials (ssh-agent, password, or private key)
    let creds = Arc::new(
        SshCredentials::try_from_str(
            "root@10.0.0.5:22",
            SshAuthenticationType::SshAgent,
        )
        .expect("invalid credentials string"),
    );

    // Reuse or open a session from the global pool
    let session = SSH_SESSIONS_POOL.get_or_create(&creds).await;

    // Execute a remote command with timeout
    let (stdout, exit) = session
        .execute_command("uname -a", Duration::from_secs(5))
        .await?;
    println!("exit={} stdout={}", exit, stdout.trim());

    session.disconnect("done").await;
    Ok(())
}
```

## File transfer
```rust
let content = session
    .download_remote_file("~/app/config.toml", Duration::from_secs(5))
    .await?;
session
    .upload_file("/tmp/hello.txt", b"hello", 0o644, Duration::from_secs(5))
    .await?;
```

## Local port forwarding

Listen locally and forward into a remote host through the SSH session.

```rust
let tunnel = session
    .start_port_forward("127.0.0.1:15432", "10.0.0.10", 5432)
    .await?;
tokio::signal::ctrl_c().await?; // keep running
tunnel.stop().await;            // abort listener/task
```

### Forwarding from a Unix socket
If the `listen_host_port` starts with `/`, a Unix socket listener is created:
```rust
let tunnel = session
    .start_port_forward("/tmp/redis.sock", "127.0.0.1", 6379)
    .await?;
```

### Managing multiple tunnels
Use `SshPortForwardTunnelsPool` to manage a set of tunnels for one session:
```rust
use my_ssh::SshPortForwardTunnelsPool;
let pool = SshPortForwardTunnelsPool::new(session.inner.clone());
pool.add_remote_connection("0.0.0.0:15432", "10.0.0.10", 5432).await?;
pool.add_remote_connection("/tmp/local.sock", "127.0.0.1", 8080).await?;
```

## Parsing `ssh://…->…` connection strings

`OverSshConnectionSettings` helps split an SSH hop from the target resource:
```rust
use my_ssh::ssh_settings::OverSshConnectionSettings;

let parsed = OverSshConnectionSettings::parse(
    "ssh://root@10.0.0.5:22->http://localhost:9200",
);
let endpoint = parsed.get_remote_endpoint(); // RemoteEndpoint for the right side
let creds = parsed.get_ssh_credentials(None).await; // Option<Arc<SshCredentials>>
println!("Remote endpoint: {:?}", endpoint);
```

You can plug secrets from an external store by implementing `SshSecurityCredentialsResolver`:
```rust
use my_ssh::ssh_settings::{SshPrivateKey, SshSecurityCredentialsResolver};

struct VaultResolver;
#[async_trait::async_trait]
impl SshSecurityCredentialsResolver for VaultResolver {
    async fn resolve_ssh_private_key(&self, _ssh_line: &str) -> Option<SshPrivateKey> {
        None
    }
    async fn resolve_ssh_password(&self, _ssh_line: &str) -> Option<String> {
        None
    }
}
```

## Auth options
- `SshAuthenticationType::SshAgent` (uses available agent keys)
- `SshAuthenticationType::UserNameAndPassword(password: String)`
- `SshAuthenticationType::PrivateKey { private_key_content, pass_phrase }`

`SshCredentials::try_from_str("user@host:22", auth_type)` is a convenient builder; defaults to port `22` when omitted.

## Errors
Most APIs return `Result<_, SshSessionError>` or `Result<_, RemotePortForwardError>`; be sure to handle network/authentication failures and timeouts appropriately.