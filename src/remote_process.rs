use russh::{Channel, ChannelMsg, ChannelWriteHalf, Sig};
use tokio::io::{AsyncWrite, AsyncWriteExt, DuplexStream};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

use crate::SshSessionError;

const STREAM_BUFFER: usize = 64 * 1024;

pub struct RemoteProcess {
    write_half: ChannelWriteHalf<russh::client::Msg>,
    pub stdout: Option<DuplexStream>,
    pub stderr: Option<DuplexStream>,
    exit_rx: Option<oneshot::Receiver<i32>>,
    _reader_task: JoinHandle<()>,
}

impl RemoteProcess {
    pub fn start(channel: Channel<russh::client::Msg>) -> Self {
        let (mut read_half, write_half) = channel.split();

        let (mut stdout_writer, stdout_reader) = tokio::io::duplex(STREAM_BUFFER);
        let (mut stderr_writer, stderr_reader) = tokio::io::duplex(STREAM_BUFFER);
        let (exit_tx, exit_rx) = oneshot::channel();
        let mut exit_tx = Some(exit_tx);

        let task = tokio::spawn(async move {
            while let Some(msg) = read_half.wait().await {
                match msg {
                    ChannelMsg::Data { ref data } => {
                        if stdout_writer.write_all(data).await.is_err() {
                            break;
                        }
                    }
                    ChannelMsg::ExtendedData { ref data, ext } => {
                        let target = if ext == 1 {
                            &mut stderr_writer
                        } else {
                            &mut stdout_writer
                        };
                        if target.write_all(data).await.is_err() {
                            break;
                        }
                    }
                    ChannelMsg::ExitStatus { exit_status } => {
                        if let Some(tx) = exit_tx.take() {
                            let _ = tx.send(exit_status as i32);
                        }
                    }
                    ChannelMsg::Close => break,
                    _ => {}
                }
            }
            let _ = stdout_writer.shutdown().await;
            let _ = stderr_writer.shutdown().await;
        });

        Self {
            write_half,
            stdout: Some(stdout_reader),
            stderr: Some(stderr_reader),
            exit_rx: Some(exit_rx),
            _reader_task: task,
        }
    }

    pub fn take_stdout(&mut self) -> Option<DuplexStream> {
        self.stdout.take()
    }

    pub fn take_stderr(&mut self) -> Option<DuplexStream> {
        self.stderr.take()
    }

    pub fn stdin(&self) -> Box<dyn AsyncWrite + Unpin + Send> {
        Box::new(self.write_half.make_writer())
    }

    pub async fn signal(&self, sig: Sig) -> Result<(), SshSessionError> {
        self.write_half.signal(sig).await?;
        Ok(())
    }

    pub async fn close_stdin(&self) -> Result<(), SshSessionError> {
        self.write_half.eof().await?;
        Ok(())
    }

    pub async fn wait_exit(&mut self) -> Result<i32, SshSessionError> {
        let rx = self
            .exit_rx
            .take()
            .ok_or_else(|| SshSessionError::Other("wait_exit already consumed".to_string()))?;
        rx.await
            .map_err(|_| SshSessionError::Other("exit channel dropped".to_string()))
    }
}
