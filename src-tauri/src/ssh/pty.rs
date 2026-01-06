use russh::{Channel, ChannelMsg};
use tauri::{AppHandle, Emitter};
use thiserror::Error;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;

/// Escape a path for use in shell commands
fn shell_escape(s: &str) -> String {
    // Wrap in single quotes and escape any single quotes in the string
    format!("'{}'", s.replace('\'', "'\\''"))
}

#[derive(Debug, Error)]
pub enum PtyError {
    #[error("Channel error: {0}")]
    ChannelError(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Terminal output event payload
#[derive(Clone, serde::Serialize)]
pub struct TerminalOutputEvent {
    pub terminal_id: String,
    pub data: Vec<u8>,
}

/// Represents an active PTY session
pub struct PtySession {
    pub terminal_id: String,
    pub connection_id: String,
    cmd_tx: mpsc::Sender<PtyCommand>,
}

enum PtyCommand {
    Write(Vec<u8>),
    Resize { cols: u32, rows: u32 },
    Close,
}

impl PtySession {
    /// Create a new PTY session
    pub fn new(
        terminal_id: String,
        connection_id: String,
        mut channel: Channel<russh::client::Msg>,
        app: AppHandle,
        working_dir: Option<String>,
        startup_command: Option<String>,
    ) -> Self {
        let (cmd_tx, mut cmd_rx) = mpsc::channel::<PtyCommand>(100);

        // Clone for the read task
        let term_id = terminal_id.clone();
        let mut channel_writer = channel.make_writer();
        let initial_dir = working_dir.clone();
        let initial_cmd = startup_command.clone();

        // Spawn a task to handle reading from the channel
        // (use Tauri's runtime for cross-platform consistency).
        tauri::async_runtime::spawn(async move {

            // Send initial cd command if working directory is specified
            if let Some(dir) = initial_dir {
                // Small delay to let shell initialize
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                // Avoid `clear` here: it destroys scrollback (especially painful with tmux/mobile).
                let cd_cmd = format!("cd {}\n", shell_escape(&dir));
                if let Err(e) = channel_writer.write_all(cd_cmd.as_bytes()).await {
                    log::error!("Failed to set initial directory: {}", e);
                }
            }

            if let Some(cmd) = initial_cmd {
                // Small delay to ensure the shell has applied the cd (and is ready for the next command).
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                let cmd = if cmd.ends_with('\n') { cmd } else { format!("{cmd}\n") };
                if let Err(e) = channel_writer.write_all(cmd.as_bytes()).await {
                    log::error!("Failed to send startup command: {}", e);
                }
            }

            loop {
                tokio::select! {
                    // Handle incoming data from the PTY
                    msg = channel.wait() => {
                        match msg {
                            None | Some(ChannelMsg::Eof) | Some(ChannelMsg::Close) => {
                                log::info!("PTY channel closed: {}", term_id);
                                break;
                            }
                            Some(ChannelMsg::Data { data }) => {
                                let data = data.to_vec();
                                let event = TerminalOutputEvent { terminal_id: term_id.clone(), data };
                                if let Err(e) = app.emit("terminal_output", event) {
                                    log::error!("Failed to emit terminal output: {}", e);
                                }
                            }
                            Some(ChannelMsg::ExtendedData { data, .. }) => {
                                let data = data.to_vec();
                                let event = TerminalOutputEvent { terminal_id: term_id.clone(), data };
                                if let Err(e) = app.emit("terminal_output", event) {
                                    log::error!("Failed to emit terminal output: {}", e);
                                }
                            }
                            // Ignore all other channel messages (requests, env, etc).
                            _ => {
                                // no-op
                            }
                        }
                    }
                    // Handle outgoing data to the PTY
                    cmd = cmd_rx.recv() => {
                        match cmd {
                            Some(PtyCommand::Write(data)) => {
                                if let Err(e) = channel_writer.write_all(&data).await {
                                    log::error!("Error writing to PTY: {}", e);
                                    let _ = channel_writer.shutdown().await;
                                    break;
                                }
                            }
                            Some(PtyCommand::Resize { cols, rows }) => {
                                // Inform the server that our window size has changed.
                                // Pixel dimensions are optional; pass 0 to avoid guessing DPI.
                                if let Err(e) = channel.window_change(cols, rows, 0, 0).await {
                                    log::warn!("PTY window change failed: {}", e);
                                }
                            }
                            Some(PtyCommand::Close) | None => {
                                let _ = channel.close().await;
                                let _ = channel_writer.shutdown().await;
                                break;
                            }
                        }
                    },
                }
            }
        });

        Self {
            terminal_id,
            connection_id,
            cmd_tx,
        }
    }

    /// Write data to the PTY
    pub async fn write(&mut self, data: &[u8]) -> Result<(), PtyError> {
        self.cmd_tx
            .send(PtyCommand::Write(data.to_vec()))
            .await
            .map_err(|e| PtyError::ChannelError(e.to_string()))?;
        Ok(())
    }

    /// Resize the PTY
    pub async fn resize(&mut self, cols: u32, rows: u32) -> Result<(), PtyError> {
        self.cmd_tx
            .send(PtyCommand::Resize { cols, rows })
            .await
            .map_err(|e| PtyError::ChannelError(e.to_string()))?;
        Ok(())
    }

    /// Close the PTY session
    pub async fn close(&mut self) -> Result<(), PtyError> {
        let _ = self.cmd_tx.send(PtyCommand::Close).await;
        Ok(())
    }
}
