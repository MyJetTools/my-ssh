```rust

#[tokio::main]
async fn main() {
    let ssh = SshPortMapServer::new("123.123.123.123:22", "root")
        .add_remote_connection("0.0.0.0:5123", "10.0.0.4", 5123)
        .add_remote_connection("0.0.0.0:6123", "10.0.0.4", 6123)
        .start();

    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
```