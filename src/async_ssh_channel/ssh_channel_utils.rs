pub fn shutdown_ssh_channel(channel: &mut ssh2::Channel) {
    let _ = channel.send_eof();
}
