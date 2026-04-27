use std::{sync::Arc, time::Duration};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{UnixListener, UnixSocket},
};

use crate::{ForwardTarget, RemotePortForwardError, SshAsyncChannel, SshSessionInnerL};

use super::SshPortForwardTunnel;

pub async fn start(
    tunnel: Arc<SshPortForwardTunnel>,
    ssh_session: Arc<SshSessionInnerL>,
) -> Result<(), RemotePortForwardError> {
    let unix_socket = UnixSocket::new_stream();

    if unix_socket.is_err() {
        return Err(RemotePortForwardError::ErrorBindingUnixSocket(format!(
            "Error creating unix socket. Err: {:?}",
            unix_socket.err()
        )));
    }

    let unix_socket = unix_socket.unwrap();

    let _ = tokio::fs::remove_file(tunnel.listen_string.as_str()).await;

    let bind_result = unix_socket.bind(tunnel.listen_string.as_str());

    if let Err(err) = &bind_result {
        return Err(RemotePortForwardError::ErrorBindingUnixSocket(format!(
            "Error binding to address: {}. Err: {:?}",
            tunnel.listen_string.as_str(),
            err
        )));
    }

    let listener = unix_socket.listen(65535);

    if let Err(err) = &listener {
        return Err(RemotePortForwardError::ErrorBindingUnixSocket(format!(
            "Error starting listen unix socket: {}. Err: {:?}",
            tunnel.listen_string.as_str(),
            err
        )));
    }

    let listener = listener.unwrap();

    let handler = tokio::spawn(server_loop(listener, tunnel.clone(), ssh_session));

    tunnel.task.lock().await.replace(handler);

    Ok(())
}

async fn server_loop(
    unix_listener: UnixListener,
    tunnel: Arc<SshPortForwardTunnel>,
    ssh_session: Arc<SshSessionInnerL>,
) {
    while tunnel.is_working() {
        let (mut socket, addr) = unix_listener.accept().await.unwrap();
        println!(
            "Accepted connection from: {:?} to serve SSH port-forward: {}->{}->{}",
            addr,
            tunnel.listen_string,
            ssh_session.credentials.to_string(),
            tunnel.target.describe(),
        );

        if !tunnel.is_working() {
            println!("Socket is being stopped. Closing connection.");
            let _ = socket.shutdown().await;
            break;
        }

        let remote_channel = open_remote(&ssh_session, &tunnel.target).await;

        if let Err(err) = remote_channel {
            println!("Error connecting to remote target: {:?}", err);
            let _ = socket.shutdown().await;
            continue;
        }

        let remote_channel = remote_channel.unwrap();

        let (ssh_reader, ssh_writer) = tokio::io::split(remote_channel);

        let (reader, writer) = socket.into_split();

        tokio::spawn(from_tcp_to_ssh_stream(tunnel.clone(), reader, ssh_writer));
        tokio::spawn(from_ssh_to_tcp_stream(tunnel.clone(), writer, ssh_reader));
    }
}

async fn open_remote(
    ssh_session: &Arc<SshSessionInnerL>,
    target: &ForwardTarget,
) -> Result<SshAsyncChannel, crate::SshSessionError> {
    let timeout = Duration::from_secs(5);
    match target {
        ForwardTarget::Tcp { host, port } => {
            ssh_session
                .open_remote_tcp_stream(host.clone(), *port, timeout)
                .await
        }
        ForwardTarget::Unix { socket_path } => {
            ssh_session
                .open_remote_unix_stream(socket_path.clone(), timeout)
                .await
        }
    }
}

async fn from_tcp_to_ssh_stream(
    tunnel: Arc<SshPortForwardTunnel>,
    mut tcp_stream: impl AsyncReadExt + Unpin,
    mut ssh_channel: tokio::io::WriteHalf<SshAsyncChannel>,
) {
    let mut buf = Vec::with_capacity(1024 * 1024);
    unsafe {
        buf.set_len(buf.capacity());
    }

    let read_timeout = Duration::from_secs(60);

    while tunnel.is_working() {
        let result = tokio::time::timeout(read_timeout, tcp_stream.read(&mut buf)).await;

        if result.is_err() {
            let _ = ssh_channel.shutdown().await;
            return;
        }

        let result = result.unwrap();

        if result.is_err() {
            let _ = ssh_channel.shutdown().await;
            return;
        }

        let size = result.unwrap();
        if size == 0 {
            let _ = ssh_channel.shutdown().await;
            return;
        }

        let result = ssh_channel.write_all(&buf[..size]).await;

        if result.is_err() {
            return;
        }
    }
}

async fn from_ssh_to_tcp_stream(
    tunnel: Arc<SshPortForwardTunnel>,
    mut tcp_writer: impl AsyncWriteExt + Unpin,
    mut ssh_channel: tokio::io::ReadHalf<SshAsyncChannel>,
) {
    let mut buf = Vec::with_capacity(1024 * 1024);
    unsafe {
        buf.set_len(buf.capacity());
    }

    let read_timeout = Duration::from_secs(60);

    while tunnel.is_working() {
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
