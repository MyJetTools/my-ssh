# my-ssh

Тонкие async-обёртки над [`russh`](https://crates.io/crates/russh) +
[`russh-sftp`](https://crates.io/crates/russh-sftp): дедуплицирующий
пул сессий, ленивый коннект, SFTP, стримы для hyper, port-forwarding,
exec one-shot и streaming.

## Install

```toml
[dependencies]
my-ssh = { tag = "${last_tag}", git = "git@github.com:MyJetTools/my-ssh.git" }
```

## Сессии

Создаём `SshCredentials` и берём сессию из глобального пула. Если для
тех же `(host, port, user)` + auth уже есть живая сессия — она
переиспользуется. Если нет или прежняя умерла — открывается новая.

```rust
use std::{sync::Arc, time::Duration};
use my_ssh::{SshAuthenticationType, SshCredentials, SSH_SESSIONS_POOL};

let creds = Arc::new(
    SshCredentials::try_from_str("root@10.0.0.5:22", SshAuthenticationType::SshAgent).unwrap(),
);
let session = SSH_SESSIONS_POOL.get_or_create(&creds).await;
```

Auth-варианты:

* `SshAuthenticationType::SshAgent` — берём ключи из `$SSH_AUTH_SOCK` (unix-only).
* `SshAuthenticationType::UserNameAndPassword(password)`.
* `SshAuthenticationType::PrivateKey { private_key_content, pass_phrase }`.

Соединение открывается лениво — первое использование сессии
триггерит handshake/auth. Heartbeat включён по умолчанию (russh
`keepalive_interval = 30s`, `keepalive_max = 3`); пул отбрасывает
сессии, у которых `russh::client::Handle::is_closed() == true`.

## Exec

### One-shot

Возвращает `(stdout, stderr, exit_code)` отдельными `Vec<u8>`:

```rust
let (stdout, stderr, exit) = session
    .execute_command("echo hi >&2; echo ok; exit 7", Duration::from_secs(5))
    .await?;
assert_eq!(exit, 7);
```

### Streaming

`start_command()` отдаёт `RemoteProcess` с раздельными stdout/stderr
(`tokio::io::AsyncRead`), stdin (`AsyncWrite`), сигналом и
ожиданием exit-кода:

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

Прервать процесс:

```rust
use my_ssh::russh::Sig;
proc.signal(Sig::TERM).await?;
```

## Файловая система (SFTP)

Все методы поддерживают `~` / `~/...` — резолвится в `$HOME`
удалённого пользователя (один `echo $HOME`, кэшируется на сессию).

### Создать папку

`mkdir -p`-семантика, идемпотентно:

```rust
session
    .create_remote_dir("~/work/data/cache", Duration::from_secs(5))
    .await?;
```

### Открыть файл (хэндл, как `tokio::fs::File`)

`open_remote_file` отдаёт `RemoteFile` (`russh_sftp::client::fs::File`),
реализующий tokio `AsyncRead + AsyncWrite + AsyncSeek`.

```rust
use my_ssh::russh_sftp::protocol::OpenFlags;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

// Запись
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

// Чтение
let mut f = session
    .open_remote_file("~/work/data/log.txt", OpenFlags::READ, None, Duration::from_secs(5))
    .await?;
let mut content = Vec::new();
f.read_to_end(&mut content).await?;
```

### Список папки

```rust
for (name, attrs) in session.list_remote_dir("~/work", Duration::from_secs(5)).await? {
    println!("{} size={:?} mode={:?}", name, attrs.size, attrs.permissions);
}
```

### Прочее

```rust
session.remove_remote_file("~/tmp/old.bin", t).await?;
session.remove_remote_dir("~/tmp/empty", t).await?;
session.rename_remote("~/a", "~/b", t).await?;
let attrs = session.remote_metadata("~/work", t).await?;
```

## Стримы (hyper-friendly)

### TCP-стрим к хосту, видимому SSH-серверу

```rust
let stream = session
    .open_remote_tcp_stream("172.17.0.2", 8080, Duration::from_secs(5))
    .await?;
// stream: AsyncRead + AsyncWrite + Unpin + Send + 'static
// можно кормить в hyper.handshake(stream).await?
```

### Unix-сокет на удалённой машине (Docker, PostgreSQL)

```rust
let stream = session
    .open_remote_unix_stream("/var/run/docker.sock", Duration::from_secs(5))
    .await?;
// тот же AsyncRead + AsyncWrite — HTTP/1.1 поверх Docker engine API
```

## Port-forwarding (client → server)

Только forward-направление: локально слушаем (TCP или Unix-socket),
тоннелируем к удалённой стороне.

### TCP-target

```rust
let tunnel = session
    .start_port_forward_to_tcp("127.0.0.1:15432", "10.0.0.10", 5432)
    .await?;
// ...
tunnel.stop().await;
```

`listen` начинающийся с `/` создаёт unix-listener:

```rust
let tunnel = session
    .start_port_forward_to_tcp("/tmp/redis.sock", "127.0.0.1", 6379)
    .await?;
```

### Unix-target (`direct-streamlocal@openssh.com`)

```rust
let tunnel = session
    .start_port_forward_to_unix("/tmp/local-docker.sock", "/var/run/docker.sock")
    .await?;
```

### Pool

Несколько туннелей на одной сессии:

```rust
use my_ssh::SshPortForwardTunnelsPool;

let pool = SshPortForwardTunnelsPool::new(session.inner.clone());
pool.add_tcp_target("127.0.0.1:15432", "10.0.0.10", 5432).await?;
pool.add_unix_target("/tmp/dock.sock", "/var/run/docker.sock").await?;
```

## Парсинг `ssh://...->...` строк

```rust
use my_ssh::ssh_settings::OverSshConnectionSettings;

let parsed = OverSshConnectionSettings::parse("ssh://root@10.0.0.5:22->http://localhost:9200");
let endpoint = parsed.get_remote_endpoint();
```

## Re-exports

Если нужна экзотика — `russh::ChannelMsg`, `russh::Sig`,
`russh_sftp::protocol::FileAttributes` и т.д. — они доступны через
`my_ssh::russh::*` и `my_ssh::russh_sftp::*` (`pub extern crate`).

## Breaking changes 0.1.x → 0.2.0

* Под капотом `russh` (pure Rust) вместо `async-ssh2-lite` / `ssh2`
  (FFI поверх libssh2). Никакого C-FFI больше нет.
* `SshAsyncChannel` теперь `russh::ChannelStream<russh::client::Msg>`
  (**tokio** `AsyncRead + AsyncWrite`). Раньше был futures-based
  канал из `async-ssh2-lite`.
* `pub extern crate ssh2` исчез. Появились `pub extern crate russh`
  и `pub extern crate russh_sftp`.
* Файловые операции теперь по SFTP (раньше по SCP). Сервер должен
  иметь включённый SFTP-subsystem (по умолчанию у sshd так и есть).
* `connect_to_remote_host(...)` → `open_remote_tcp_stream(...)`.
  Добавлен `open_remote_unix_stream(...)`.
* `start_port_forward(...)` → `start_port_forward_to_tcp(...)` /
  `start_port_forward_to_unix(...)`. У `SshPortForwardTunnelsPool`:
  `add_remote_connection` → `add_tcp_target` / `add_unix_target`.
* `download_remote_file` / `upload_file` удалены — заменены на
  низкоуровневый `open_remote_file`, дальше пишет/читает пользователь
  через tokio I/O.
* `execute_command(cmd, timeout) -> (String, i32)` →
  `(Vec<u8>, Vec<u8>, i32)` — stdout и stderr раздельно, в байтах.
* Добавлены: `start_command` (streaming exec, `RemoteProcess`),
  `create_remote_dir`, `list_remote_dir`, `remove_remote_file`,
  `remove_remote_dir`, `rename_remote`, `remote_metadata`.
* `SshSession::is_connected()` → `is_alive() -> async bool`.
  Опирается на `Handle::is_closed()`.
* SSH heartbeat включён по умолчанию.
* Unix-only (`SshAgent` использует `connect_env()` через
  `$SSH_AUTH_SOCK`; Windows pageant пока не покрыт).

## Errors

Все методы возвращают `Result<_, SshSessionError>` (или
`RemotePortForwardError` у port-forward). Прозрачные варианты
оборачивают upstream-ошибки:

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
