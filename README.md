# my-ssh

Thin async wrappers over [`russh`](https://crates.io/crates/russh) +
[`russh-sftp`](https://crates.io/crates/russh-sftp): a deduplicating
session pool, lazy connect, SFTP, hyper-friendly streams,
port-forwarding, one-shot and streaming exec.

For internals (pool behavior, re-exports, error model, migration from
0.1.x) see [docs/WHAT_INSIDE.md](docs/WHAT_INSIDE.md).

## Install

```toml
[dependencies]
my-ssh = { tag = "${last_tag}", git = "git@github.com:MyJetTools/my-ssh.git" }
```

## Sessions

Build `SshCredentials` and pull a session from the global pool. The same
`(host, port, user)` + auth combo reuses the live session; otherwise a
new one is opened.

```rust
use std::{sync::Arc, time::Duration};
use my_ssh::{SshAuthenticationType, SshCredentials, SSH_SESSIONS_POOL};

let creds = Arc::new(
    SshCredentials::try_from_str("root@10.0.0.5:22", SshAuthenticationType::SshAgent).unwrap(),
);
let session = SSH_SESSIONS_POOL.get_or_create(&creds).await;
```

Auth variants:

* `SshAuthenticationType::SshAgent` — keys from `$SSH_AUTH_SOCK` (unix-only).
* `SshAuthenticationType::UserNameAndPassword(password)`.
* `SshAuthenticationType::PrivateKey { private_key_content, pass_phrase }`.

## Exec

### One-shot

Returns `(stdout, stderr, exit_code)` as separate `Vec<u8>`:

```rust
let (stdout, stderr, exit) = session
    .execute_command("echo hi >&2; echo ok; exit 7", Duration::from_secs(5))
    .await?;
assert_eq!(exit, 7);
```

### Streaming

`start_command()` returns a `RemoteProcess` with separate stdout/stderr
(`tokio::io::AsyncRead`), stdin (`AsyncWrite`), a signal channel, and
exit-code wait:

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

Interrupt the process:

```rust
use my_ssh::russh::Sig;
proc.signal(Sig::TERM).await?;
```

## Filesystem (SFTP)

All methods accept `~` / `~/...` — resolved against the remote user's
`$HOME` (one `echo $HOME`, cached per session).

### Create a directory

`mkdir -p` semantics, idempotent:

```rust
session
    .create_remote_dir("~/work/data/cache", Duration::from_secs(5))
    .await?;
```

### Open a file (handle, like `tokio::fs::File`)

`open_remote_file` returns a `RemoteFile`
(`russh_sftp::client::fs::File`) implementing tokio
`AsyncRead + AsyncWrite + AsyncSeek`.

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

### Misc

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
// can be fed into hyper.handshake(stream).await?
```

### Unix socket on the remote machine (Docker, PostgreSQL)

```rust
let stream = session
    .open_remote_unix_stream("/var/run/docker.sock", Duration::from_secs(5))
    .await?;
// same AsyncRead + AsyncWrite — HTTP/1.1 over the Docker engine API
```

## Port-forwarding (client → server)

Forward direction only: listen locally (TCP or unix-socket), tunnel to
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

Multiple tunnels on a single session:

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
