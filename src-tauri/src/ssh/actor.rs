use crate::ssh::client::{SshConnection, SshError};
use crate::ssh::pty::PtySession;
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::sync::{mpsc, oneshot};

pub struct ConnectionActorHandle {
    pub tx: mpsc::Sender<ConnectionRequest>,
    pub task: tauri::async_runtime::JoinHandle<()>,
}

pub enum ConnectionRequest {
    GetHomeDir {
        respond_to: oneshot::Sender<Result<String, SshError>>,
    },
    ListDir {
        path: String,
        respond_to: oneshot::Sender<Result<Vec<crate::ssh::sftp::SftpEntry>, SshError>>,
    },
    ReadFile {
        path: String,
        respond_to: oneshot::Sender<Result<String, SshError>>,
    },
    WriteFile {
        path: String,
        content: String,
        respond_to: oneshot::Sender<Result<(), SshError>>,
    },
    Stat {
        path: String,
        respond_to: oneshot::Sender<Result<crate::ssh::sftp::SftpStat, SshError>>,
    },
    CreateFile {
        path: String,
        respond_to: oneshot::Sender<Result<(), SshError>>,
    },
    CreateDir {
        path: String,
        respond_to: oneshot::Sender<Result<(), SshError>>,
    },
    Delete {
        path: String,
        respond_to: oneshot::Sender<Result<(), SshError>>,
    },
    Rename {
        old_path: String,
        new_path: String,
        respond_to: oneshot::Sender<Result<(), SshError>>,
    },
    CreatePty {
        terminal_id: String,
        working_dir: Option<String>,
        respond_to: oneshot::Sender<Result<PtySession, SshError>>,
    },
    Disconnect {
        respond_to: oneshot::Sender<Result<(), SshError>>,
    },
}

#[derive(Clone, Serialize)]
struct ConnectionStatusEvent {
    connection_id: String,
    status: String,
    detail: Option<String>,
}

pub fn spawn_connection_actor(
    app: AppHandle,
    connection_id: String,
    mut connection: SshConnection,
) -> ConnectionActorHandle {
    let (tx, mut rx) = mpsc::channel::<ConnectionRequest>(64);

    let task = tauri::async_runtime::spawn(async move {
        let _ = app.emit(
            "connection_status_changed",
            ConnectionStatusEvent {
                connection_id: connection_id.clone(),
                status: "connected".to_string(),
                detail: None,
            },
        );

        let mut disconnect_reason: Option<String> = None;

        while let Some(request) = rx.recv().await {
            match request {
                ConnectionRequest::GetHomeDir { respond_to } => {
                    let result = connection.get_home_dir().await;
                    if let Err(e) = &result {
                        if is_fatal_connection_error(e) {
                            disconnect_reason = Some(e.to_string());
                        }
                    }
                    let _ = respond_to.send(result);
                }
                ConnectionRequest::ListDir { path, respond_to } => {
                    let result = connection.list_dir(&path).await;
                    if let Err(e) = &result {
                        if is_fatal_connection_error(e) {
                            disconnect_reason = Some(e.to_string());
                        }
                    }
                    let _ = respond_to.send(result);
                }
                ConnectionRequest::ReadFile { path, respond_to } => {
                    let result = connection.read_file(&path).await;
                    if let Err(e) = &result {
                        if is_fatal_connection_error(e) {
                            disconnect_reason = Some(e.to_string());
                        }
                    }
                    let _ = respond_to.send(result);
                }
                ConnectionRequest::WriteFile {
                    path,
                    content,
                    respond_to,
                } => {
                    let result = connection.write_file(&path, &content).await;
                    if let Err(e) = &result {
                        if is_fatal_connection_error(e) {
                            disconnect_reason = Some(e.to_string());
                        }
                    }
                    let _ = respond_to.send(result);
                }
                ConnectionRequest::Stat { path, respond_to } => {
                    let result = connection.stat(&path).await;
                    if let Err(e) = &result {
                        if is_fatal_connection_error(e) {
                            disconnect_reason = Some(e.to_string());
                        }
                    }
                    let _ = respond_to.send(result);
                }
                ConnectionRequest::CreateFile { path, respond_to } => {
                    let result = connection.create_file(&path).await;
                    if let Err(e) = &result {
                        if is_fatal_connection_error(e) {
                            disconnect_reason = Some(e.to_string());
                        }
                    }
                    let _ = respond_to.send(result);
                }
                ConnectionRequest::CreateDir { path, respond_to } => {
                    let result = connection.create_dir(&path).await;
                    if let Err(e) = &result {
                        if is_fatal_connection_error(e) {
                            disconnect_reason = Some(e.to_string());
                        }
                    }
                    let _ = respond_to.send(result);
                }
                ConnectionRequest::Delete { path, respond_to } => {
                    let result = connection.delete(&path).await;
                    if let Err(e) = &result {
                        if is_fatal_connection_error(e) {
                            disconnect_reason = Some(e.to_string());
                        }
                    }
                    let _ = respond_to.send(result);
                }
                ConnectionRequest::Rename {
                    old_path,
                    new_path,
                    respond_to,
                } => {
                    let result = connection.rename(&old_path, &new_path).await;
                    if let Err(e) = &result {
                        if is_fatal_connection_error(e) {
                            disconnect_reason = Some(e.to_string());
                        }
                    }
                    let _ = respond_to.send(result);
                }
                ConnectionRequest::CreatePty {
                    terminal_id,
                    working_dir,
                    respond_to,
                } => {
                    let result = connection
                        .create_pty_session(
                            terminal_id.clone(),
                            connection_id.clone(),
                            app.clone(),
                            working_dir,
                        )
                        .await;
                    if let Err(e) = &result {
                        if is_fatal_connection_error(e) {
                            disconnect_reason = Some(e.to_string());
                        }
                    }
                    let _ = respond_to.send(result);
                }
                ConnectionRequest::Disconnect { respond_to } => {
                    let result = connection.disconnect().await;
                    let _ = respond_to.send(result);
                    disconnect_reason = Some("User requested disconnect".to_string());
                    break;
                }
            }

            if disconnect_reason.is_some() {
                break;
            }
        }

        let _ = app.emit(
            "connection_status_changed",
            ConnectionStatusEvent {
                connection_id,
                status: "disconnected".to_string(),
                detail: disconnect_reason,
            },
        );
    });

    ConnectionActorHandle { tx, task }
}

fn is_fatal_connection_error(error: &SshError) -> bool {
    match error {
        SshError::ConnectionFailed(_) => true,
        SshError::AuthenticationFailed(_) => true,
        SshError::ChannelError(_) => true,
        // Timeouts and SFTP-level issues may be transient; caller can retry.
        SshError::SftpTimeout | SshError::SftpSessionClosed | SshError::SftpError(_) => false,
        SshError::IoError(_) => true,
    }
}
