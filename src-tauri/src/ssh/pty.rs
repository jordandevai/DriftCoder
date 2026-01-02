use russh::Channel;
use tauri::{AppHandle, Emitter};
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
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
    Close,
}

impl PtySession {
    /// Create a new PTY session
    pub fn new(
        terminal_id: String,
        connection_id: String,
        channel: Channel<russh::client::Msg>,
        app: AppHandle,
        working_dir: Option<String>,
    ) -> Self {
        let (cmd_tx, mut cmd_rx) = mpsc::channel::<PtyCommand>(100);

        // Clone for the read task
        let term_id = terminal_id.clone();
        let mut channel_stream = channel.into_stream();
        let initial_dir = working_dir.clone();

        // Spawn a task to handle reading from the channel
        // (use Tauri's runtime for cross-platform consistency).
        tauri::async_runtime::spawn(async move {
            let mut buffer = vec![0u8; 4096];

            // Send initial cd command if working directory is specified
            if let Some(dir) = initial_dir {
                // Small delay to let shell initialize
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                let cd_cmd = format!("cd {} && clear\n", shell_escape(&dir));
                if let Err(e) = channel_stream.write_all(cd_cmd.as_bytes()).await {
                    log::error!("Failed to set initial directory: {}", e);
                }
            }

            loop {
                tokio::select! {
                    // Handle incoming data from the PTY
                    result = channel_stream.read(&mut buffer) => {
                        match result {
                            Ok(0) => {
                                // Channel closed
                                log::info!("PTY channel closed: {}", term_id);
                                break;
                            }
                            Ok(n) => {
                                let data = buffer[..n].to_vec();
                                let event = TerminalOutputEvent {
                                    terminal_id: term_id.clone(),
                                    data,
                                };
                                if let Err(e) = app.emit("terminal_output", event) {
                                    log::error!("Failed to emit terminal output: {}", e);
                                }
                            }
                            Err(e) => {
                                log::error!("Error reading from PTY: {}", e);
                                break;
                            }
                        }
                    }
                    // Handle outgoing data to the PTY
                    cmd = cmd_rx.recv() => {
                        match cmd {
                            Some(PtyCommand::Write(data)) => {
                                if let Err(e) = channel_stream.write_all(&data).await {
                                    log::error!("Error writing to PTY: {}", e);
                                    let _ = channel_stream.shutdown().await;
                                    break;
                                }
                            }
                            Some(PtyCommand::Close) | None => {
                                let _ = channel_stream.shutdown().await;
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
        // Note: russh channel window change would be called here
        // For now, this is a placeholder - resize requires channel access
        log::info!(
            "PTY resize requested: {}x{} for {}",
            cols,
            rows,
            self.terminal_id
        );
        Ok(())
    }

    /// Close the PTY session
    pub async fn close(&mut self) -> Result<(), PtyError> {
        let _ = self.cmd_tx.send(PtyCommand::Close).await;
        Ok(())
    }
}
