use crate::ssh::actor::ConnectionRequest;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
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

/// List directory contents
#[tauri::command]
pub async fn sftp_list_dir(
    state: State<'_, Arc<Mutex<AppState>>>,
    conn_id: String,
    path: String,
) -> Result<Vec<FileEntry>, String> {
    let tx = {
        let app_state = state.lock().await;
        app_state
            .get_connection_sender(&conn_id)
            .ok_or("Connection not found")?
    };

    let (respond_to, rx) = oneshot::channel();
    tx.send(ConnectionRequest::ListDir {
        path: path.clone(),
        respond_to,
    })
    .await
    .map_err(|_| "Connection is closed".to_string())?;

    let entries = rx
        .await
        .map_err(|_| "Connection is closed".to_string())?
        .map_err(|e| format!("SFTP list directory failed for '{}': {}", path, e))?;

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

/// Read a file's contents
#[tauri::command]
pub async fn sftp_read_file(
    state: State<'_, Arc<Mutex<AppState>>>,
    conn_id: String,
    path: String,
) -> Result<String, String> {
    let tx = {
        let app_state = state.lock().await;
        app_state
            .get_connection_sender(&conn_id)
            .ok_or("Connection not found")?
    };

    let (respond_to, rx) = oneshot::channel();
    tx.send(ConnectionRequest::ReadFile {
        path: path.clone(),
        respond_to,
    })
    .await
    .map_err(|_| "Connection is closed".to_string())?;

    rx.await
        .map_err(|_| "Connection is closed".to_string())?
        .map_err(|e| format!("SFTP read file failed for '{}': {}", path, e))
}

/// Write content to a file
#[tauri::command]
pub async fn sftp_write_file(
    state: State<'_, Arc<Mutex<AppState>>>,
    conn_id: String,
    path: String,
    content: String,
) -> Result<FileMeta, String> {
    let tx = {
        let app_state = state.lock().await;
        app_state
            .get_connection_sender(&conn_id)
            .ok_or("Connection not found")?
    };

    let (respond_to, rx) = oneshot::channel();
    tx.send(ConnectionRequest::WriteFile {
        path: path.clone(),
        content,
        respond_to,
    })
    .await
    .map_err(|_| "Connection is closed".to_string())?;

    rx.await
        .map_err(|_| "Connection is closed".to_string())?
        .map_err(|e| format!("SFTP write file failed for '{}': {}", path, e))?;

    let (respond_to, rx) = oneshot::channel();
    tx.send(ConnectionRequest::Stat {
        path: path.clone(),
        respond_to,
    })
    .await
    .map_err(|_| "Connection is closed".to_string())?;

    let stat = rx
        .await
        .map_err(|_| "Connection is closed".to_string())?
        .map_err(|e| format!("SFTP stat failed for '{}': {}", path, e))?;

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
) -> Result<FileMeta, String> {
    let tx = {
        let app_state = state.lock().await;
        app_state
            .get_connection_sender(&conn_id)
            .ok_or("Connection not found")?
    };

    let (respond_to, rx) = oneshot::channel();
    tx.send(ConnectionRequest::Stat {
        path: path.clone(),
        respond_to,
    })
    .await
    .map_err(|_| "Connection is closed".to_string())?;

    let stat = rx
        .await
        .map_err(|_| "Connection is closed".to_string())?
        .map_err(|e| format!("SFTP stat failed for '{}': {}", path, e))?;

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
) -> Result<(), String> {
    let tx = {
        let app_state = state.lock().await;
        app_state
            .get_connection_sender(&conn_id)
            .ok_or("Connection not found")?
    };

    let (respond_to, rx) = oneshot::channel();
    tx.send(ConnectionRequest::CreateFile {
        path: path.clone(),
        respond_to,
    })
    .await
    .map_err(|_| "Connection is closed".to_string())?;

    rx.await
        .map_err(|_| "Connection is closed".to_string())?
        .map_err(|e| format!("SFTP create file failed for '{}': {}", path, e))
}

/// Create a new directory
#[tauri::command]
pub async fn sftp_create_dir(
    state: State<'_, Arc<Mutex<AppState>>>,
    conn_id: String,
    path: String,
) -> Result<(), String> {
    let tx = {
        let app_state = state.lock().await;
        app_state
            .get_connection_sender(&conn_id)
            .ok_or("Connection not found")?
    };

    let (respond_to, rx) = oneshot::channel();
    tx.send(ConnectionRequest::CreateDir {
        path: path.clone(),
        respond_to,
    })
    .await
    .map_err(|_| "Connection is closed".to_string())?;

    rx.await
        .map_err(|_| "Connection is closed".to_string())?
        .map_err(|e| format!("SFTP create directory failed for '{}': {}", path, e))
}

/// Delete a file or directory
#[tauri::command]
pub async fn sftp_delete(
    state: State<'_, Arc<Mutex<AppState>>>,
    conn_id: String,
    path: String,
) -> Result<(), String> {
    let tx = {
        let app_state = state.lock().await;
        app_state
            .get_connection_sender(&conn_id)
            .ok_or("Connection not found")?
    };

    let (respond_to, rx) = oneshot::channel();
    tx.send(ConnectionRequest::Delete {
        path: path.clone(),
        respond_to,
    })
    .await
    .map_err(|_| "Connection is closed".to_string())?;

    rx.await
        .map_err(|_| "Connection is closed".to_string())?
        .map_err(|e| format!("SFTP delete failed for '{}': {}", path, e))
}

/// Rename/move a file or directory
#[tauri::command]
pub async fn sftp_rename(
    state: State<'_, Arc<Mutex<AppState>>>,
    conn_id: String,
    old_path: String,
    new_path: String,
) -> Result<(), String> {
    let tx = {
        let app_state = state.lock().await;
        app_state
            .get_connection_sender(&conn_id)
            .ok_or("Connection not found")?
    };

    let (respond_to, rx) = oneshot::channel();
    tx.send(ConnectionRequest::Rename {
        old_path: old_path.clone(),
        new_path: new_path.clone(),
        respond_to,
    })
    .await
    .map_err(|_| "Connection is closed".to_string())?;

    rx.await
        .map_err(|_| "Connection is closed".to_string())?
        .map_err(|e| format!("SFTP rename failed ('{}' -> '{}'): {}", old_path, new_path, e))
}
