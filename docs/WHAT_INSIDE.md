# What's inside

Internal design of `my-ssh`: the session pool, connect behavior, what is
re-exported, the error model, and migration from 0.1.x.

## Session pool

`SSH_SESSIONS_POOL.get_or_create(&creds)` deduplicates sessions by
`(host, port, user)` + auth. If a live session for the same key already
exists — it's reused. If not, or the previous one died — a new session
is opened.

The connection is opened **lazily** — the first use of the session
triggers the handshake/auth.

Heartbeat is enabled by default (russh `keepalive_interval = 30s`,
`keepalive_max = 3`); the pool drops sessions whose
`russh::client::Handle::is_closed() == true`.

`SshSession::is_alive() -> async bool` is backed by
`Handle::is_closed()`.

## `~` resolution

All SFTP methods accept `~` / `~/...` — resolved against the remote
user's `$HOME` via a single `echo $HOME`; the result is cached per
session.

## Re-exports

For exotic needs — `russh::ChannelMsg`, `russh::Sig`,
`russh_sftp::protocol::FileAttributes`, etc. — they are reachable via
`my_ssh::russh::*` and `my_ssh::russh_sftp::*` (`pub extern crate`).

## Errors

All methods return `Result<_, SshSessionError>` (or
`RemotePortForwardError` for port-forward). Transparent variants wrap
upstream errors:

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

## Breaking changes 0.1.x → 0.2.0

* Backed by `russh` (pure Rust) instead of `async-ssh2-lite` / `ssh2`
  (FFI over libssh2). No more C-FFI.
* `SshAsyncChannel` is now `russh::ChannelStream<russh::client::Msg>`
  (**tokio** `AsyncRead + AsyncWrite`). Previously a futures-based
  channel from `async-ssh2-lite`.
* `pub extern crate ssh2` is gone. Replaced by `pub extern crate russh`
  and `pub extern crate russh_sftp`.
* Filesystem operations now go over SFTP (previously SCP). The server
  must have the SFTP subsystem enabled (default for sshd).
* `connect_to_remote_host(...)` → `open_remote_tcp_stream(...)`.
  Added `open_remote_unix_stream(...)`.
* `start_port_forward(...)` → `start_port_forward_to_tcp(...)` /
  `start_port_forward_to_unix(...)`. On `SshPortForwardTunnelsPool`:
  `add_remote_connection` → `add_tcp_target` / `add_unix_target`.
* `download_remote_file` / `upload_file` are removed — replaced with
  the lower-level `open_remote_file`; the caller reads/writes via
  tokio I/O.
* `execute_command(cmd, timeout) -> (String, i32)` →
  `(Vec<u8>, Vec<u8>, i32)` — stdout and stderr split, in bytes.
* Added: `start_command` (streaming exec, `RemoteProcess`),
  `create_remote_dir`, `list_remote_dir`, `remove_remote_file`,
  `remove_remote_dir`, `rename_remote`, `remote_metadata`.
* `SshSession::is_connected()` → `is_alive() -> async bool`. Backed by
  `Handle::is_closed()`.
* SSH heartbeat is enabled by default.
* Unix-only (`SshAgent` uses `connect_env()` via `$SSH_AUTH_SOCK`;
  Windows pageant is not yet covered).
