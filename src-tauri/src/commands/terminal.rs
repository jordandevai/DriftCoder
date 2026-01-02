use crate::ssh::actor::ConnectionRequest;
use crate::state::AppState;
use crate::ipc_error::IpcError;
use serde_json::json;
use std::sync::Arc;
use tauri::{AppHandle, State};
use tokio::sync::Mutex;
use tokio::sync::oneshot;
use uuid::Uuid;

/// Create a new terminal session
#[tauri::command]
pub async fn terminal_create(
    _app: AppHandle,
    state: State<'_, Arc<Mutex<AppState>>>,
    conn_id: String,
    working_dir: Option<String>,
) -> Result<String, IpcError> {
    let terminal_id = Uuid::new_v4().to_string();
    let working_dir_for_context = working_dir.clone();

    let tx = {
        let app_state = state.lock().await;
        app_state
            .get_connection_sender(&conn_id)
            .ok_or_else(|| IpcError::new("connection_not_found", "Connection not found"))?
    };

    let (respond_to, rx) = oneshot::channel();
    tx.send(ConnectionRequest::CreatePty {
        terminal_id: terminal_id.clone(),
        working_dir,
        respond_to,
    })
    .await
    .map_err(|_| IpcError::new("connection_closed", "Connection is closed"))?;

    let pty_session = rx
        .await
        .map_err(|_| IpcError::new("connection_closed", "Connection is closed"))?
        .map_err(|e| {
            IpcError::new("terminal_create_failed", "Terminal create failed")
                .with_raw(e.to_string())
                .with_context(json!({ "workingDir": working_dir_for_context }))
        })?;

    let mut app_state = state.lock().await;
    app_state.add_terminal(terminal_id.clone(), pty_session);

    log::info!("Terminal session created: {}", terminal_id);

    Ok(terminal_id)
}

/// Write data to a terminal
#[tauri::command]
pub async fn terminal_write(
    state: State<'_, Arc<Mutex<AppState>>>,
    term_id: String,
    data: Vec<u8>,
) -> Result<(), IpcError> {
    let mut app_state = state.lock().await;

    let write_result = {
        let terminal = app_state
            .get_terminal_mut(&term_id)
            .ok_or_else(|| IpcError::new("terminal_not_found", "Terminal not found"))?;
        terminal.write(&data).await
    };

    if let Err(e) = write_result {
        // If the PTY task has ended (mpsc channel closed), drop the terminal so subsequent calls
        // become `terminal_not_found` instead of spamming repeated write failures.
        let raw = e.to_string();
        if raw.to_lowercase().contains("channel closed") {
            let _ = app_state.remove_terminal(&term_id);
        }

        return Err(
            IpcError::new("terminal_write_failed", "Terminal write failed")
                .with_raw(raw)
                .with_context(json!({ "terminalId": term_id })),
        );
    }

    Ok(())
}

/// Resize a terminal
#[tauri::command]
pub async fn terminal_resize(
    state: State<'_, Arc<Mutex<AppState>>>,
    term_id: String,
    cols: u32,
    rows: u32,
) -> Result<(), IpcError> {
    let mut app_state = state.lock().await;

    let terminal = app_state
        .get_terminal_mut(&term_id)
        .ok_or_else(|| IpcError::new("terminal_not_found", "Terminal not found"))?;

    terminal
        .resize(cols, rows)
        .await
        .map_err(|e| {
            IpcError::new("terminal_resize_failed", "Terminal resize failed")
                .with_raw(e.to_string())
                .with_context(json!({ "terminalId": term_id, "cols": cols, "rows": rows }))
        })?;

    Ok(())
}

/// Close a terminal session
#[tauri::command]
pub async fn terminal_close(
    state: State<'_, Arc<Mutex<AppState>>>,
    term_id: String,
) -> Result<(), IpcError> {
    let mut app_state = state.lock().await;

    if let Some(mut terminal) = app_state.remove_terminal(&term_id) {
        terminal.close().await.map_err(|e| {
            IpcError::new("terminal_close_failed", "Terminal close failed")
                .with_raw(e.to_string())
                .with_context(json!({ "terminalId": term_id }))
        })?;
        log::info!("Terminal session closed: {}", term_id);
    }

    Ok(())
}
