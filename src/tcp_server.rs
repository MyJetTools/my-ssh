use std::{sync::Arc, time::Duration};

use tokio::{io::AsyncWriteExt, net::TcpListener};

use crate::{ssh_credentials, SshAsyncChannel};

use super::SshRemoteConnection;

pub fn start(
    remote_connection: Arc<SshRemoteConnection>,
    ssh_credentials: Arc<ssh_credentials::SshCredentials>,
) {
    tokio::spawn(server_loop(remote_connection, ssh_credentials));
}

async fn server_loop(
    remote_connection: Arc<SshRemoteConnection>,
    ssh_credentials: Arc<ssh_credentials::SshCredentials>,
) {
    let listener = TcpListener::bind(remote_connection.listen_host_port.as_str()).await;

    if let Err(err) = &listener {
        println!(
            "Error binding to address: {}. Err: {:?}",
            remote_connection.listen_host_port.as_str(),
            err
        );
        return;
    }

    let listener = listener.unwrap();

    loop {
        let (mut socket, addr) = listener.accept().await.unwrap();
        println!("Accepted connection from: {:?}", addr);

        let ssh_session = crate::SSH_SESSION_POOL
            .get_or_create_ssh_session(&ssh_credentials)
            .await;

        let remote_channel = ssh_session
            .connect_to_remote_host(&remote_connection.remote_host, Duration::from_secs(5))
            .await;

        if let Err(err) = remote_channel {
            println!("Error connecting to remote host: {:?}", err);
            let _ = socket.shutdown().await;
            ssh_session.disconnect().await;
            continue;
        }

        let remote_channel = remote_channel.unwrap();

        let (ssh_reader, ssh_writer) = futures::AsyncReadExt::split(remote_channel);

        let (reader, writer) = socket.into_split();

        tokio::spawn(from_tcp_to_ssh_stream(reader, ssh_writer));
        tokio::spawn(from_ssh_to_tcp_stream(writer, ssh_reader));
    }
}

async fn from_tcp_to_ssh_stream(
    mut tcp_stream: impl tokio::io::AsyncReadExt + Unpin,
    mut ssh_channel: futures::io::WriteHalf<SshAsyncChannel>,
) {
    use futures::AsyncWriteExt;
    let mut buf = Vec::with_capacity(1024 * 1024);
    unsafe {
        buf.set_len(buf.capacity());
    }

    loop {
        let result = tcp_stream.read(&mut buf).await;
        if result.is_err() {
            let _ = ssh_channel.close().await;
            return;
        }

        let size = result.unwrap();
        if size == 0 {
            let _ = ssh_channel.close().await;
            return;
        }

        let result = ssh_channel.write(&buf[..size]).await;

        if result.is_err() {
            return;
        }
    }
}

async fn from_ssh_to_tcp_stream(
    mut tcp_writer: impl tokio::io::AsyncWriteExt + Unpin,
    mut ssh_channel: futures::io::ReadHalf<SshAsyncChannel>,
) {
    use futures::AsyncReadExt;

    let mut buf = Vec::with_capacity(1024 * 1024);
    unsafe {
        buf.set_len(buf.capacity());
    }

    loop {
        let result = ssh_channel.read(&mut buf).await;

        if result.is_err() {
            let _ = tcp_writer.shutdown().await;
            return;
        }

        let size = result.unwrap();

        if size == 0 {
            let _ = tcp_writer.shutdown().await;
            return;
        }

        let result = tcp_writer.write_all(&buf[..size]).await;
        if result.is_err() {
            return;
        }
    }
}
