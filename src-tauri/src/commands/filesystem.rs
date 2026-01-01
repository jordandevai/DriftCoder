use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

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
    let mut app_state = state.lock().await;

    let connection = app_state
        .get_connection_mut(&conn_id)
        .ok_or("Connection not found")?;

    let entries = connection
        .list_dir(&path)
        .await
        .map_err(|e| e.to_string())?;

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
    let mut app_state = state.lock().await;

    let connection = app_state
        .get_connection_mut(&conn_id)
        .ok_or("Connection not found")?;

    let content = connection
        .read_file(&path)
        .await
        .map_err(|e| e.to_string())?;

    Ok(content)
}

/// Write content to a file
#[tauri::command]
pub async fn sftp_write_file(
    state: State<'_, Arc<Mutex<AppState>>>,
    conn_id: String,
    path: String,
    content: String,
) -> Result<FileMeta, String> {
    let mut app_state = state.lock().await;

    let connection = app_state
        .get_connection_mut(&conn_id)
        .ok_or("Connection not found")?;

    connection
        .write_file(&path, &content)
        .await
        .map_err(|e| e.to_string())?;

    // Get updated file metadata
    let stat = connection.stat(&path).await.map_err(|e| e.to_string())?;

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
    let mut app_state = state.lock().await;

    let connection = app_state
        .get_connection_mut(&conn_id)
        .ok_or("Connection not found")?;

    let stat = connection.stat(&path).await.map_err(|e| e.to_string())?;

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
    let mut app_state = state.lock().await;

    let connection = app_state
        .get_connection_mut(&conn_id)
        .ok_or("Connection not found")?;

    connection
        .create_file(&path)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Create a new directory
#[tauri::command]
pub async fn sftp_create_dir(
    state: State<'_, Arc<Mutex<AppState>>>,
    conn_id: String,
    path: String,
) -> Result<(), String> {
    let mut app_state = state.lock().await;

    let connection = app_state
        .get_connection_mut(&conn_id)
        .ok_or("Connection not found")?;

    connection
        .create_dir(&path)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Delete a file or directory
#[tauri::command]
pub async fn sftp_delete(
    state: State<'_, Arc<Mutex<AppState>>>,
    conn_id: String,
    path: String,
) -> Result<(), String> {
    let mut app_state = state.lock().await;

    let connection = app_state
        .get_connection_mut(&conn_id)
        .ok_or("Connection not found")?;

    connection.delete(&path).await.map_err(|e| e.to_string())?;

    Ok(())
}

/// Rename/move a file or directory
#[tauri::command]
pub async fn sftp_rename(
    state: State<'_, Arc<Mutex<AppState>>>,
    conn_id: String,
    old_path: String,
    new_path: String,
) -> Result<(), String> {
    let mut app_state = state.lock().await;

    let connection = app_state
        .get_connection_mut(&conn_id)
        .ok_or("Connection not found")?;

    connection
        .rename(&old_path, &new_path)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}
