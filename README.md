# my-ssh

Thin async wrappers over [`russh`](https://crates.io/crates/russh) +
[`russh-sftp`](https://crates.io/crates/russh-sftp): a deduplicating
session pool, lazy connect, SFTP, hyper-friendly streams, port forwarding,
and both one-shot and streaming exec.

## Install

```toml
[dependencies]
my-ssh = { tag = "${last_tag}", git = "git@github.com:MyJetTools/my-ssh.git" }
```

## Sessions

Build `SshCredentials` and take a session from the global pool. If a live
session already exists for the same `(host, port, user)` + auth, it is
reused. If there is none, or the previous one died, a new one is opened.

```rust
use std::{sync::Arc, time::Duration};
use my_ssh::{SshAuthenticationType, SshCredentials, SSH_SESSIONS_POOL};

let creds = Arc::new(
    SshCredentials::try_from_str("root@10.0.0.5:22", SshAuthenticationType::SshAgent).unwrap(),
);
let session = SSH_SESSIONS_POOL.get_or_create(&creds).await;
```

Auth options:

* `SshAuthenticationType::SshAgent` - keys are taken from `$SSH_AUTH_SOCK` (unix-only).
* `SshAuthenticationType::UserNameAndPassword(password)`.
* `SshAuthenticationType::PrivateKey { private_key_content, pass_phrase }`.

The connection is opened lazily - the first use of a session triggers the
handshake/auth. Heartbeat is on by default (russh
`keepalive_interval = 30s`, `keepalive_max = 3`); the pool discards
sessions whose `russh::client::Handle::is_closed()` is `true`.

## Exec

### One-shot

Returns `(stdout, stderr, exit_code)` as separate `Vec<u8>` values:

```rust
let (stdout, stderr, exit) = session
    .execute_command("echo hi >&2; echo ok; exit 7", Duration::from_secs(5))
    .await?;
assert_eq!(exit, 7);
```

### Streaming

`start_command()` hands back a `RemoteProcess` with separate stdout/stderr
(`tokio::io::AsyncRead`), stdin (`AsyncWrite`), a signal method, and a way
to await the exit code:

```rust
use tokio::io::{AsyncReadExt, AsyncWriteExt};

let mut proc = session.start_command("grep ERROR").await?;

// stdin
{
    let mut stdin = proc.stdin();
    stdin.write_all(b"line one\nERROR boom\nlast\n").await?;
    stdin.shutdown().await?; // EOF
}

// stdout
let mut out = proc.take_stdout().unwrap();
let mut buf = String::new();
out.read_to_string(&mut buf).await?;

let exit = proc.wait_exit().await?;
```

Terminating the process:

```rust
use my_ssh::russh::Sig;
proc.signal(Sig::TERM).await?;
```

## File system (SFTP)

Every method accepts `~` / `~/...`, which resolves to the remote user's
`$HOME` (a single `echo $HOME`, cached per session).

### Create a directory

`mkdir -p` semantics, idempotent:

```rust
session
    .create_remote_dir("~/work/data/cache", Duration::from_secs(5))
    .await?;
```

### Open a file (a handle, like `tokio::fs::File`)

`open_remote_file` returns a `RemoteFile` (`russh_sftp::client::fs::File`)
implementing tokio `AsyncRead + AsyncWrite + AsyncSeek`.

```rust
use my_ssh::russh_sftp::protocol::OpenFlags;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

// Write
let mut f = session
    .open_remote_file(
        "~/work/data/log.txt",
        OpenFlags::WRITE | OpenFlags::CREATE | OpenFlags::TRUNCATE,
        Some(0o644),
        Duration::from_secs(5),
    )
    .await?;
f.write_all(b"hello\n").await?;
f.flush().await?;
f.shutdown().await?;

// Read
let mut f = session
    .open_remote_file("~/work/data/log.txt", OpenFlags::READ, None, Duration::from_secs(5))
    .await?;
let mut content = Vec::new();
f.read_to_end(&mut content).await?;
```

### List a directory

```rust
for (name, attrs) in session.list_remote_dir("~/work", Duration::from_secs(5)).await? {
    println!("{} size={:?} mode={:?}", name, attrs.size, attrs.permissions);
}
```

### Everything else

```rust
session.remove_remote_file("~/tmp/old.bin", t).await?;
session.remove_remote_dir("~/tmp/empty", t).await?;
session.rename_remote("~/a", "~/b", t).await?;
let attrs = session.remote_metadata("~/work", t).await?;
```

## Streams (hyper-friendly)

### TCP stream to a host visible from the SSH server

```rust
let stream = session
    .open_remote_tcp_stream("172.17.0.2", 8080, Duration::from_secs(5))
    .await?;
// stream: AsyncRead + AsyncWrite + Unpin + Send + 'static
// can be fed straight into hyper.handshake(stream).await?
```

### Unix socket on the remote machine (Docker, PostgreSQL)

```rust
let stream = session
    .open_remote_unix_stream("/var/run/docker.sock", Duration::from_secs(5))
    .await?;
// the same AsyncRead + AsyncWrite - HTTP/1.1 over the Docker engine API
```

## Port forwarding (client to server)

Forward direction only: listen locally (TCP or unix socket) and tunnel to
the remote side.

### TCP target

```rust
let tunnel = session
    .start_port_forward_to_tcp("127.0.0.1:15432", "10.0.0.10", 5432)
    .await?;
// ...
tunnel.stop().await;
```

A `listen` value starting with `/` creates a unix listener:

```rust
let tunnel = session
    .start_port_forward_to_tcp("/tmp/redis.sock", "127.0.0.1", 6379)
    .await?;
```

### Unix target (`direct-streamlocal@openssh.com`)

```rust
let tunnel = session
    .start_port_forward_to_unix("/tmp/local-docker.sock", "/var/run/docker.sock")
    .await?;
```

### Pool

Several tunnels over a single session:

```rust
use my_ssh::SshPortForwardTunnelsPool;

let pool = SshPortForwardTunnelsPool::new(session.inner.clone());
pool.add_tcp_target("127.0.0.1:15432", "10.0.0.10", 5432).await?;
pool.add_unix_target("/tmp/dock.sock", "/var/run/docker.sock").await?;
```

## Parsing `ssh://...->...` strings

```rust
use my_ssh::ssh_settings::OverSshConnectionSettings;

let parsed = OverSshConnectionSettings::parse("ssh://root@10.0.0.5:22->http://localhost:9200");
let endpoint = parsed.get_remote_endpoint();
```

## Re-exports

When something exotic is needed - `russh::ChannelMsg`, `russh::Sig`,
`russh_sftp::protocol::FileAttributes` and so on - it is reachable through
`my_ssh::russh::*` and `my_ssh::russh_sftp::*` (`pub extern crate`).

## Breaking changes 0.1.x to 0.2.0

* `russh` (pure Rust) under the hood instead of `async-ssh2-lite` / `ssh2`
  (FFI over libssh2). There is no C FFI left.
* `SshAsyncChannel` is now `russh::ChannelStream<russh::client::Msg>`
  (**tokio** `AsyncRead + AsyncWrite`). It used to be a futures-based
  channel from `async-ssh2-lite`.
* `pub extern crate ssh2` is gone. `pub extern crate russh` and
  `pub extern crate russh_sftp` took its place.
* File operations now go over SFTP (they used to go over SCP). The server
  must have the SFTP subsystem enabled, which is the sshd default.
* `connect_to_remote_host(...)` became `open_remote_tcp_stream(...)`.
  `open_remote_unix_stream(...)` was added.
* `start_port_forward(...)` became `start_port_forward_to_tcp(...)` /
  `start_port_forward_to_unix(...)`. On `SshPortForwardTunnelsPool`:
  `add_remote_connection` became `add_tcp_target` / `add_unix_target`.
* `download_remote_file` / `upload_file` were removed - replaced by the
  low-level `open_remote_file`, after which the caller reads and writes
  through tokio I/O.
* `execute_command(cmd, timeout) -> (String, i32)` became
  `(Vec<u8>, Vec<u8>, i32)` - stdout and stderr separately, as bytes.
* Added: `start_command` (streaming exec, `RemoteProcess`),
  `create_remote_dir`, `list_remote_dir`, `remove_remote_file`,
  `remove_remote_dir`, `rename_remote`, `remote_metadata`.
* `SshSession::is_connected()` became `is_alive() -> async bool`, backed
  by `Handle::is_closed()`.
* SSH heartbeat is on by default.
* Unix-only (`SshAgent` uses `connect_env()` through `$SSH_AUTH_SOCK`;
  Windows pageant is not covered yet).

## Errors

Every method returns `Result<_, SshSessionError>` (or
`RemotePortForwardError` for port forwarding). Transparent variants wrap
the upstream errors:

```rust
pub enum SshSessionError {
    SshSessionIsNotActive,
    StdIoStreamError(std::io::Error),
    SshError(russh::Error),
    SshKeysError(russh::keys::Error),
    SftpError(russh_sftp::client::error::Error),
    SshAuthenticationError,
    Other(String),
    Timeout,
}
```
