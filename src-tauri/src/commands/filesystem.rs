use crate::ipc_error::IpcError;
use crate::ssh::actor::ConnectionRequest;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;
use tokio::sync::oneshot;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_directory: bool,
    pub size: u64,
    pub mtime: i64,
    pub permissions: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileMeta {
    pub path: String,
    pub size: u64,
    pub mtime: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileReadResult {
    pub path: String,
    pub content: String,
    pub size: u64,
    pub mtime: i64,
}

/// List directory contents
#[tauri::command]
pub async fn sftp_list_dir(
    state: State<'_, Arc<Mutex<AppState>>>,
    conn_id: String,
    path: String,
) -> Result<Vec<FileEntry>, IpcError> {
    let tx = {
        let app_state = state.lock().await;
        app_state
            .get_connection_sender(&conn_id)
            .ok_or_else(|| IpcError::new("connection_not_found", "Connection not found"))?
    };

    let (respond_to, rx) = oneshot::channel();
    tx.send(ConnectionRequest::ListDir {
        path: path.clone(),
        respond_to,
    })
    .await
    .map_err(|_| IpcError::new("connection_closed", "Connection is closed"))?;

    let entries = rx
        .await
        .map_err(|_| IpcError::new("connection_closed", "Connection is closed"))?
        .map_err(|e| {
            IpcError::new("sftp_list_dir_failed", "SFTP list directory failed")
                .with_raw(e.to_string())
                .with_context(json!({ "path": path }))
        })?;

    let file_entries: Vec<FileEntry> = entries
        .into_iter()
        .filter(|e| e.name != "." && e.name != "..")
        .map(|e| FileEntry {
            name: e.name.clone(),
            path: if path.ends_with('/') {
                format!("{}{}", path, e.name)
            } else {
                format!("{}/{}", path, e.name)
            },
            is_directory: e.is_directory,
            size: e.size,
            mtime: e.mtime,
            permissions: e.permissions,
        })
        .collect();

    Ok(file_entries)
}

/// Read a file and its stat (single IPC call)
#[tauri::command]
pub async fn sftp_read_file_with_stat(
    state: State<'_, Arc<Mutex<AppState>>>,
    conn_id: String,
    path: String,
) -> Result<FileReadResult, IpcError> {
    let tx = {
        let app_state = state.lock().await;
        app_state
            .get_connection_sender(&conn_id)
            .ok_or_else(|| IpcError::new("connection_not_found", "Connection not found"))?
    };

    let (respond_to, rx) = oneshot::channel();
    tx.send(ConnectionRequest::ReadFileWithStat {
        path: path.clone(),
        respond_to,
    })
    .await
    .map_err(|_| IpcError::new("connection_closed", "Connection is closed"))?;

    let (content, stat) = rx
        .await
        .map_err(|_| IpcError::new("connection_closed", "Connection is closed"))?
        .map_err(|e| {
            IpcError::new("sftp_read_file_failed", "SFTP read file failed")
                .with_raw(e.to_string())
                .with_context(json!({ "path": path }))
        })?;

    Ok(FileReadResult {
        path,
        content,
        size: stat.size,
        mtime: stat.mtime,
    })
}

/// Read a file's contents
#[tauri::command]
pub async fn sftp_read_file(
    state: State<'_, Arc<Mutex<AppState>>>,
    conn_id: String,
    path: String,
) -> Result<String, IpcError> {
    let tx = {
        let app_state = state.lock().await;
        app_state
            .get_connection_sender(&conn_id)
            .ok_or_else(|| IpcError::new("connection_not_found", "Connection not found"))?
    };

    let (respond_to, rx) = oneshot::channel();
    tx.send(ConnectionRequest::ReadFile {
        path: path.clone(),
        respond_to,
    })
    .await
    .map_err(|_| IpcError::new("connection_closed", "Connection is closed"))?;

    rx.await
        .map_err(|_| IpcError::new("connection_closed", "Connection is closed"))?
        .map_err(|e| {
            IpcError::new("sftp_read_file_failed", "SFTP read file failed")
                .with_raw(e.to_string())
                .with_context(json!({ "path": path }))
        })
}

/// Write content to a file
#[tauri::command]
pub async fn sftp_write_file(
    state: State<'_, Arc<Mutex<AppState>>>,
    conn_id: String,
    path: String,
    content: String,
) -> Result<FileMeta, IpcError> {
    let tx = {
        let app_state = state.lock().await;
        app_state
            .get_connection_sender(&conn_id)
            .ok_or_else(|| IpcError::new("connection_not_found", "Connection not found"))?
    };

    let (respond_to, rx) = oneshot::channel();
    tx.send(ConnectionRequest::WriteFile {
        path: path.clone(),
        content,
        respond_to,
    })
    .await
    .map_err(|_| IpcError::new("connection_closed", "Connection is closed"))?;

    rx.await
        .map_err(|_| IpcError::new("connection_closed", "Connection is closed"))?
        .map_err(|e| {
            IpcError::new("sftp_write_file_failed", "SFTP write file failed")
                .with_raw(e.to_string())
                .with_context(json!({ "path": path }))
        })?;

    let (respond_to, rx) = oneshot::channel();
    tx.send(ConnectionRequest::Stat {
        path: path.clone(),
        respond_to,
    })
    .await
    .map_err(|_| IpcError::new("connection_closed", "Connection is closed"))?;

    let stat = rx
        .await
        .map_err(|_| IpcError::new("connection_closed", "Connection is closed"))?
        .map_err(|e| {
            IpcError::new("sftp_stat_failed", "SFTP stat failed")
                .with_raw(e.to_string())
                .with_context(json!({ "path": path }))
        })?;

    Ok(FileMeta {
        path,
        size: stat.size,
        mtime: stat.mtime,
    })
}

/// Get file metadata
#[tauri::command]
pub async fn sftp_stat(
    state: State<'_, Arc<Mutex<AppState>>>,
    conn_id: String,
    path: String,
) -> Result<FileMeta, IpcError> {
    let tx = {
        let app_state = state.lock().await;
        app_state
            .get_connection_sender(&conn_id)
            .ok_or_else(|| IpcError::new("connection_not_found", "Connection not found"))?
    };

    let (respond_to, rx) = oneshot::channel();
    tx.send(ConnectionRequest::Stat {
        path: path.clone(),
        respond_to,
    })
    .await
    .map_err(|_| IpcError::new("connection_closed", "Connection is closed"))?;

    let stat = rx
        .await
        .map_err(|_| IpcError::new("connection_closed", "Connection is closed"))?
        .map_err(|e| {
            IpcError::new("sftp_stat_failed", "SFTP stat failed")
                .with_raw(e.to_string())
                .with_context(json!({ "path": path }))
        })?;

    Ok(FileMeta {
        path,
        size: stat.size,
        mtime: stat.mtime,
    })
}

/// Create a new empty file
#[tauri::command]
pub async fn sftp_create_file(
    state: State<'_, Arc<Mutex<AppState>>>,
    conn_id: String,
    path: String,
) -> Result<(), IpcError> {
    let tx = {
        let app_state = state.lock().await;
        app_state
            .get_connection_sender(&conn_id)
            .ok_or_else(|| IpcError::new("connection_not_found", "Connection not found"))?
    };

    let (respond_to, rx) = oneshot::channel();
    tx.send(ConnectionRequest::CreateFile {
        path: path.clone(),
        respond_to,
    })
    .await
    .map_err(|_| IpcError::new("connection_closed", "Connection is closed"))?;

    rx.await
        .map_err(|_| IpcError::new("connection_closed", "Connection is closed"))?
        .map_err(|e| {
            IpcError::new("sftp_create_file_failed", "SFTP create file failed")
                .with_raw(e.to_string())
                .with_context(json!({ "path": path }))
        })
}

/// Create a new directory
#[tauri::command]
pub async fn sftp_create_dir(
    state: State<'_, Arc<Mutex<AppState>>>,
    conn_id: String,
    path: String,
) -> Result<(), IpcError> {
    let tx = {
        let app_state = state.lock().await;
        app_state
            .get_connection_sender(&conn_id)
            .ok_or_else(|| IpcError::new("connection_not_found", "Connection not found"))?
    };

    let (respond_to, rx) = oneshot::channel();
    tx.send(ConnectionRequest::CreateDir {
        path: path.clone(),
        respond_to,
    })
    .await
    .map_err(|_| IpcError::new("connection_closed", "Connection is closed"))?;

    rx.await
        .map_err(|_| IpcError::new("connection_closed", "Connection is closed"))?
        .map_err(|e| {
            IpcError::new("sftp_create_dir_failed", "SFTP create directory failed")
                .with_raw(e.to_string())
                .with_context(json!({ "path": path }))
        })
}

/// Delete a file or directory
#[tauri::command]
pub async fn sftp_delete(
    state: State<'_, Arc<Mutex<AppState>>>,
    conn_id: String,
    path: String,
) -> Result<(), IpcError> {
    let tx = {
        let app_state = state.lock().await;
        app_state
            .get_connection_sender(&conn_id)
            .ok_or_else(|| IpcError::new("connection_not_found", "Connection not found"))?
    };

    let (respond_to, rx) = oneshot::channel();
    tx.send(ConnectionRequest::Delete {
        path: path.clone(),
        respond_to,
    })
    .await
    .map_err(|_| IpcError::new("connection_closed", "Connection is closed"))?;

    rx.await
        .map_err(|_| IpcError::new("connection_closed", "Connection is closed"))?
        .map_err(|e| {
            IpcError::new("sftp_delete_failed", "SFTP delete failed")
                .with_raw(e.to_string())
                .with_context(json!({ "path": path }))
        })
}

/// Rename/move a file or directory
#[tauri::command]
pub async fn sftp_rename(
    state: State<'_, Arc<Mutex<AppState>>>,
    conn_id: String,
    old_path: String,
    new_path: String,
) -> Result<(), IpcError> {
    let tx = {
        let app_state = state.lock().await;
        app_state
            .get_connection_sender(&conn_id)
            .ok_or_else(|| IpcError::new("connection_not_found", "Connection not found"))?
    };

    let (respond_to, rx) = oneshot::channel();
    tx.send(ConnectionRequest::Rename {
        old_path: old_path.clone(),
        new_path: new_path.clone(),
        respond_to,
    })
    .await
    .map_err(|_| IpcError::new("connection_closed", "Connection is closed"))?;

    rx.await
        .map_err(|_| IpcError::new("connection_closed", "Connection is closed"))?
        .map_err(|e| {
            IpcError::new("sftp_rename_failed", "SFTP rename failed")
                .with_raw(e.to_string())
                .with_context(json!({ "oldPath": old_path, "newPath": new_path }))
        })
}
