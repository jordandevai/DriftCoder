use crate::ssh::client::{SshConnection, SshError};
use crate::ssh::pty::PtySession;
use crate::trace::{emit_trace, TraceEvent};
use serde::Serialize;
use std::collections::HashMap;
use std::time::{Duration, Instant};
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
    ReadFileWithStat {
        path: String,
        respond_to: oneshot::Sender<Result<(String, crate::ssh::sftp::SftpStat), SshError>>,
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
#[serde(rename_all = "camelCase")]
struct ConnectionStatusEvent {
    connection_id: String,
    status: String,
    detail: Option<String>,
}

const LIST_DIR_TIMEOUT: Duration = Duration::from_secs(45);
const READ_FILE_TIMEOUT: Duration = Duration::from_secs(60);
const READ_FILE_WITH_STAT_TIMEOUT: Duration = Duration::from_secs(75);
const WRITE_FILE_TIMEOUT: Duration = Duration::from_secs(60);
const STAT_TIMEOUT: Duration = Duration::from_secs(30);
const MUTATION_TIMEOUT: Duration = Duration::from_secs(30);
const PTY_TIMEOUT: Duration = Duration::from_secs(20);

const DIR_CACHE_TTL: Duration = Duration::from_secs(10);
const DIR_CACHE_MAX_ENTRIES: usize = 128;

pub fn spawn_connection_actor(
    app: AppHandle,
    connection_id: String,
    mut connection: SshConnection,
) -> ConnectionActorHandle {
    let (tx, mut rx) = mpsc::channel::<ConnectionRequest>(64);

    let task = tauri::async_runtime::spawn(async move {
        let mut dir_cache = DirectoryCache::new(DIR_CACHE_TTL, DIR_CACHE_MAX_ENTRIES);

        emit_trace(&app, TraceEvent::new("actor", "loop_start", &format!("Actor loop starting for {}", connection_id)));

        let _ = app.emit(
            "connection_status_changed",
            ConnectionStatusEvent {
                connection_id: connection_id.clone(),
                status: "connected".to_string(),
                detail: None,
            },
        );

        let mut disconnect_reason: Option<String> = None;
        let mut request_count = 0u64;

        emit_trace(&app, TraceEvent::new("actor", "waiting", "Actor waiting for requests"));

        while let Some(request) = rx.recv().await {
            request_count += 1;
            let request_name = match &request {
                ConnectionRequest::GetHomeDir { .. } => "GetHomeDir",
                ConnectionRequest::ListDir { path, .. } => {
                    emit_trace(&app, TraceEvent::new("actor", "list_dir", &format!("ListDir request: {}", path)));
                    "ListDir"
                }
                ConnectionRequest::ReadFileWithStat { path, .. } => {
                    emit_trace(&app, TraceEvent::new("actor", "read_file_stat", &format!("ReadFileWithStat: {}", path)));
                    "ReadFileWithStat"
                }
                ConnectionRequest::ReadFile { path, .. } => {
                    emit_trace(&app, TraceEvent::new("actor", "read_file", &format!("ReadFile: {}", path)));
                    "ReadFile"
                }
                ConnectionRequest::WriteFile { path, .. } => {
                    emit_trace(&app, TraceEvent::new("actor", "write_file", &format!("WriteFile: {}", path)));
                    "WriteFile"
                }
                ConnectionRequest::Stat { path, .. } => {
                    emit_trace(&app, TraceEvent::new("actor", "stat", &format!("Stat: {}", path)));
                    "Stat"
                }
                ConnectionRequest::CreateFile { .. } => "CreateFile",
                ConnectionRequest::CreateDir { .. } => "CreateDir",
                ConnectionRequest::Delete { .. } => "Delete",
                ConnectionRequest::Rename { .. } => "Rename",
                ConnectionRequest::CreatePty { .. } => "CreatePty",
                ConnectionRequest::Disconnect { .. } => {
                    emit_trace(&app, TraceEvent::new("actor", "disconnect_req", "Disconnect request received"));
                    "Disconnect"
                }
            };
            emit_trace(&app, TraceEvent::new("actor", "request", &format!("Request #{}: {}", request_count, request_name)));

            match request {
                ConnectionRequest::GetHomeDir { respond_to } => {
                    let result = match tokio::time::timeout(STAT_TIMEOUT, connection.get_home_dir()).await {
                        Ok(r) => r,
                        Err(_) => {
                            connection.reset_sftp();
                            Err(SshError::SftpTimeout)
                        }
                    };
                    if let Err(e) = &result {
                        if is_fatal_connection_error(e) {
                            disconnect_reason = Some(e.to_string());
                        }
                    }
                    let _ = respond_to.send(result);
                }
                ConnectionRequest::ListDir { path, respond_to } => {
                    let cache_key = normalize_dir_path(&path);
                    if let Some(cached) = dir_cache.get(&cache_key) {
                        let _ = respond_to.send(Ok(cached));
                        continue;
                    }

                    let result = match tokio::time::timeout(LIST_DIR_TIMEOUT, connection.list_dir(&path)).await {
                        Ok(r) => r,
                        Err(_) => {
                            connection.reset_sftp();
                            Err(SshError::SftpTimeout)
                        }
                    };
                    if let Err(e) = &result {
                        if is_fatal_connection_error(e) {
                            disconnect_reason = Some(e.to_string());
                        }
                    } else if let Ok(entries) = &result {
                        dir_cache.put(cache_key, entries.clone());
                    }
                    let _ = respond_to.send(result);
                }
                ConnectionRequest::ReadFileWithStat { path, respond_to } => {
                    let result = match tokio::time::timeout(
                        READ_FILE_WITH_STAT_TIMEOUT,
                        connection.read_file_with_stat(&path),
                    )
                    .await
                    {
                        Ok(r) => r,
                        Err(_) => {
                            connection.reset_sftp();
                            Err(SshError::SftpTimeout)
                        }
                    };
                    if let Err(e) = &result {
                        if is_fatal_connection_error(e) {
                            disconnect_reason = Some(e.to_string());
                        }
                    }
                    let _ = respond_to.send(result);
                }
                ConnectionRequest::ReadFile { path, respond_to } => {
                    let result =
                        match tokio::time::timeout(READ_FILE_TIMEOUT, connection.read_file(&path)).await
                        {
                            Ok(r) => r,
                            Err(_) => {
                                connection.reset_sftp();
                                Err(SshError::SftpTimeout)
                            }
                        };
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
                    let result = match tokio::time::timeout(
                        WRITE_FILE_TIMEOUT,
                        connection.write_file(&path, &content),
                    )
                    .await
                    {
                        Ok(r) => r,
                        Err(_) => {
                            connection.reset_sftp();
                            Err(SshError::SftpTimeout)
                        }
                    };
                    if let Err(e) = &result {
                        if is_fatal_connection_error(e) {
                            disconnect_reason = Some(e.to_string());
                        }
                    } else {
                        dir_cache.invalidate_parent_of_path(&path);
                    }
                    let _ = respond_to.send(result);
                }
                ConnectionRequest::Stat { path, respond_to } => {
                    let result = match tokio::time::timeout(STAT_TIMEOUT, connection.stat(&path)).await {
                        Ok(r) => r,
                        Err(_) => {
                            connection.reset_sftp();
                            Err(SshError::SftpTimeout)
                        }
                    };
                    if let Err(e) = &result {
                        if is_fatal_connection_error(e) {
                            disconnect_reason = Some(e.to_string());
                        }
                    }
                    let _ = respond_to.send(result);
                }
                ConnectionRequest::CreateFile { path, respond_to } => {
                    let result =
                        match tokio::time::timeout(MUTATION_TIMEOUT, connection.create_file(&path)).await
                        {
                            Ok(r) => r,
                            Err(_) => {
                                connection.reset_sftp();
                                Err(SshError::SftpTimeout)
                            }
                        };
                    if let Err(e) = &result {
                        if is_fatal_connection_error(e) {
                            disconnect_reason = Some(e.to_string());
                        }
                    } else {
                        dir_cache.invalidate_parent_of_path(&path);
                    }
                    let _ = respond_to.send(result);
                }
                ConnectionRequest::CreateDir { path, respond_to } => {
                    let result =
                        match tokio::time::timeout(MUTATION_TIMEOUT, connection.create_dir(&path)).await
                        {
                            Ok(r) => r,
                            Err(_) => {
                                connection.reset_sftp();
                                Err(SshError::SftpTimeout)
                            }
                        };
                    if let Err(e) = &result {
                        if is_fatal_connection_error(e) {
                            disconnect_reason = Some(e.to_string());
                        }
                    } else {
                        dir_cache.invalidate_parent_of_path(&path);
                    }
                    let _ = respond_to.send(result);
                }
                ConnectionRequest::Delete { path, respond_to } => {
                    let result =
                        match tokio::time::timeout(MUTATION_TIMEOUT, connection.delete(&path)).await {
                            Ok(r) => r,
                            Err(_) => {
                                connection.reset_sftp();
                                Err(SshError::SftpTimeout)
                            }
                        };
                    if let Err(e) = &result {
                        if is_fatal_connection_error(e) {
                            disconnect_reason = Some(e.to_string());
                        }
                    } else {
                        dir_cache.invalidate_path_and_parent(&path);
                    }
                    let _ = respond_to.send(result);
                }
                ConnectionRequest::Rename {
                    old_path,
                    new_path,
                    respond_to,
                } => {
                    let result = match tokio::time::timeout(
                        MUTATION_TIMEOUT,
                        connection.rename(&old_path, &new_path),
                    )
                    .await
                    {
                        Ok(r) => r,
                        Err(_) => {
                            connection.reset_sftp();
                            Err(SshError::SftpTimeout)
                        }
                    };
                    if let Err(e) = &result {
                        if is_fatal_connection_error(e) {
                            disconnect_reason = Some(e.to_string());
                        }
                    } else {
                        dir_cache.invalidate_parent_of_path(&old_path);
                        dir_cache.invalidate_parent_of_path(&new_path);
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
                        );
                    let result = match tokio::time::timeout(PTY_TIMEOUT, result).await {
                        Ok(r) => r,
                        Err(_) => Err(SshError::ChannelError("PTY request timed out".to_string())),
                    };
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
                emit_trace(&app, TraceEvent::new("actor", "breaking", &format!("Breaking due to disconnect: {:?}", disconnect_reason)).error());
                break;
            }
        }

        // Loop exited - either channel closed or disconnect requested
        if disconnect_reason.is_none() {
            emit_trace(&app, TraceEvent::new("actor", "channel_closed", &format!("Actor channel closed (no senders) after {} requests", request_count)).error());
            disconnect_reason = Some("Channel closed (all senders dropped)".to_string());
        }

        emit_trace(&app, TraceEvent::new("actor", "loop_exit", &format!("Actor loop exiting: {:?}", disconnect_reason)));

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
        SshError::DnsLookupFailed { .. } => true,
        SshError::TcpConnectFailed { .. } => true,
        SshError::TcpConnectTimeout { .. } => true,
        SshError::HandshakeFailed { .. } => true,
        SshError::HandshakeJoinAborted { .. } => true,
        SshError::HostKeyUntrusted { .. } => true,
        SshError::HostKeyMismatch { .. } => true,
        SshError::ConnectionFailed(_) => true,
        SshError::AuthenticationFailed(_) => true,
        SshError::ChannelError(_) => true,
        // Timeouts and SFTP-level issues may be transient; caller can retry.
        SshError::SftpTimeout | SshError::SftpSessionClosed | SshError::SftpError(_) => false,
        SshError::IoError(_) => true,
    }
}

struct DirectoryCache {
    ttl: Duration,
    max_entries: usize,
    entries: HashMap<String, (Instant, Vec<crate::ssh::sftp::SftpEntry>)>,
}

impl DirectoryCache {
    fn new(ttl: Duration, max_entries: usize) -> Self {
        Self {
            ttl,
            max_entries,
            entries: HashMap::new(),
        }
    }

    fn get(&mut self, path: &str) -> Option<Vec<crate::ssh::sftp::SftpEntry>> {
        let now = Instant::now();
        match self.entries.get(path) {
            Some((created_at, entries)) if now.duration_since(*created_at) <= self.ttl => {
                Some(entries.clone())
            }
            Some(_) => {
                self.entries.remove(path);
                None
            }
            None => None,
        }
    }

    fn put(&mut self, path: String, entries: Vec<crate::ssh::sftp::SftpEntry>) {
        self.entries.insert(path, (Instant::now(), entries));
        self.evict_if_needed();
    }

    fn invalidate(&mut self, path: &str) {
        self.entries.remove(path);
    }

    fn invalidate_parent_of_path(&mut self, path: &str) {
        if let Some(parent) = parent_dir(path) {
            self.invalidate(&parent);
        }
    }

    fn invalidate_path_and_parent(&mut self, path: &str) {
        let normalized = normalize_dir_path(path);
        self.invalidate(&normalized);
        self.invalidate_parent_of_path(path);
    }

    fn evict_if_needed(&mut self) {
        while self.entries.len() > self.max_entries {
            if let Some((oldest_key, _)) = self
                .entries
                .iter()
                .min_by_key(|(_, (created_at, _))| *created_at)
                .map(|(k, v)| (k.clone(), v.0))
            {
                self.entries.remove(&oldest_key);
            } else {
                break;
            }
        }
    }
}

fn normalize_dir_path(path: &str) -> String {
    if path == "/" {
        return "/".to_string();
    }
    path.trim_end_matches('/').to_string()
}

fn parent_dir(path: &str) -> Option<String> {
    let normalized = normalize_dir_path(path);
    if normalized == "/" {
        return None;
    }
    let mut parts = normalized.split('/').filter(|p| !p.is_empty()).collect::<Vec<_>>();
    parts.pop();
    if parts.is_empty() {
        Some("/".to_string())
    } else {
        Some(format!("/{}", parts.join("/")))
    }
}
