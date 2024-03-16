pub async fn connect(
    ssh_session: &ssh2::Session,
    host: &str,
    port: u16,
) -> Result<ssh2::Channel, ssh2::Error> {
    let channel = super::await_would_block::await_would_block(|| {
        ssh_session.channel_direct_tcpip(host, port, None)
    })
    .await?;

    Ok(channel)
}
