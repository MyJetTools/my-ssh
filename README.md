```rust

#[tokio::main]
async fn main() {
    let ssh = SshRemoteServer::new("12.12.13.13:22", "root")
            .add_remote_connection("0.0.0.0:5123", "10.0.0.4", 5123)
            .add_remote_connection("0.0.0.0:6123", "10.0.0.4", 6123)
            .add_remote_connection("0.0.0.0:33000", "10.0.0.5", 33000)
            .add_remote_connection("0.0.0.0:32999", "10.0.0.4", 33000)
            .start();

        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
}
```