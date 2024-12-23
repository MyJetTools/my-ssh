use std::{sync::Arc, time::Duration};

use tokio::{io::AsyncWriteExt, net::TcpListener};

use crate::{RemotePortForwardError, SshAsyncChannel, SshSessionInnerL};

use super::SshPortForwardTunnel;

pub async fn start(
    remote_connection: Arc<SshPortForwardTunnel>,
    ssh_session: Arc<SshSessionInnerL>,
) -> Result<(), RemotePortForwardError> {
    let listener = TcpListener::bind(remote_connection.listen_string.as_str()).await;

    if let Err(err) = &listener {
        return Err(RemotePortForwardError::CanNotBindListenEndpoint(format!(
            "Error binding to address: {}. Err: {:?}",
            remote_connection.listen_string.as_str(),
            err
        )));
    }

    let listener = listener.unwrap();
    let handler = tokio::spawn(server_loop(
        listener,
        remote_connection.clone(),
        ssh_session,
    ));

    remote_connection.task.lock().await.replace(handler);

    Ok(())
}

async fn server_loop(
    listener: TcpListener,
    remote_connection: Arc<SshPortForwardTunnel>,
    ssh_session: Arc<SshSessionInnerL>,
) {
    while remote_connection.is_working() {
        let (mut socket, addr) = listener.accept().await.unwrap();
        println!(
            "Accepted connection from: {:?} to serve SSH port-forward: {}->{}->{}:{}",
            addr,
            remote_connection.listen_string,
            ssh_session.credentials.to_string(),
            remote_connection.remote_host,
            remote_connection.remote_port
        );

        let remote_channel = ssh_session
            .connect_to_remote_host(
                &remote_connection.remote_host,
                remote_connection.remote_port,
                Duration::from_secs(5),
            )
            .await;

        if let Err(err) = remote_channel {
            println!("Error connecting to remote host: {:?}", err);
            let _ = socket.shutdown().await;
            continue;
        }

        let remote_channel = remote_channel.unwrap();

        let (ssh_reader, ssh_writer) = futures::AsyncReadExt::split(remote_channel);

        let (reader, writer) = socket.into_split();

        tokio::spawn(from_tcp_to_ssh_stream(
            remote_connection.clone(),
            reader,
            ssh_writer,
        ));
        tokio::spawn(from_ssh_to_tcp_stream(
            remote_connection.clone(),
            writer,
            ssh_reader,
        ));
    }
}

async fn from_tcp_to_ssh_stream(
    remote_connection: Arc<SshPortForwardTunnel>,
    mut tcp_stream: impl tokio::io::AsyncReadExt + Unpin,
    mut ssh_channel: futures::io::WriteHalf<SshAsyncChannel>,
) {
    use futures::AsyncWriteExt;
    let mut buf = Vec::with_capacity(1024 * 1024);
    unsafe {
        buf.set_len(buf.capacity());
    }

    let read_timeout = Duration::from_secs(60);

    while remote_connection.is_working() {
        let result = tokio::time::timeout(read_timeout, tcp_stream.read(&mut buf)).await;

        if result.is_err() {
            let _ = ssh_channel.close().await;
            return;
        }

        let result = result.unwrap();

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
    remote_connection: Arc<SshPortForwardTunnel>,
    mut tcp_writer: impl tokio::io::AsyncWriteExt + Unpin,
    mut ssh_channel: futures::io::ReadHalf<SshAsyncChannel>,
) {
    use futures::AsyncReadExt;

    let mut buf = Vec::with_capacity(1024 * 1024);
    unsafe {
        buf.set_len(buf.capacity());
    }

    let read_timeout = Duration::from_secs(60);

    while remote_connection.is_working() {
        let result = tokio::time::timeout(read_timeout, ssh_channel.read(&mut buf)).await;

        if result.is_err() {
            let _ = tcp_writer.shutdown().await;
            return;
        }

        let result = result.unwrap();

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
