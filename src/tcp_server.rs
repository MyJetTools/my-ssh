use std::sync::Arc;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        *,
    },
};

use super::{SshPortMapChannel, SshRemoteConnection};

pub fn start(remote_connection: Arc<SshRemoteConnection>) {
    tokio::spawn(server_loop(remote_connection));
}

async fn server_loop(remote_connection: Arc<SshRemoteConnection>) {
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

        let remote_channel = remote_connection
            .ssh_session
            .connect_to_remote_host(
                &remote_connection.remote_host,
                remote_connection.remote_port,
            )
            .await;

        if let Err(err) = remote_channel {
            println!("Error connecting to remote host: {:?}", err);
            let _ = socket.shutdown().await;
            remote_connection.ssh_session.disconnect().await;
            continue;
        }

        let remote_channel = remote_channel.unwrap();

        let remote_channel = Arc::new(remote_channel);

        let (reader, writer) = socket.into_split();

        tokio::spawn(from_tcp_to_ssh_stream(reader, remote_channel.clone()));
        tokio::spawn(from_ssh_to_tcp_stream(writer, remote_channel.clone()));
    }
}

async fn from_tcp_to_ssh_stream(
    mut tcp_stream: OwnedReadHalf,
    ssh_channel: Arc<SshPortMapChannel>,
) {
    let mut buf = Vec::with_capacity(1024 * 1024);
    unsafe {
        buf.set_len(buf.capacity());
    }

    loop {
        let result = tcp_stream.read(&mut buf).await;
        if result.is_err() {
            ssh_channel.close().await;
            return;
        }

        let size = result.unwrap();
        if size == 0 {
            ssh_channel.close().await;
            return;
        }

        let result = ssh_channel.write_to_channel(&buf[..size]).await;

        if result.is_err() {
            return;
        }
    }
}

async fn from_ssh_to_tcp_stream(
    mut tcp_writer: OwnedWriteHalf,
    ssh_channel: Arc<SshPortMapChannel>,
) {
    let mut buf = Vec::with_capacity(1024 * 1024);
    unsafe {
        buf.set_len(buf.capacity());
    }

    loop {
        let result = ssh_channel.read_from_channel(&mut buf).await;

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
