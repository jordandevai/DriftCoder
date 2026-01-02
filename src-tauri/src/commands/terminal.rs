use crate::ssh::actor::ConnectionRequest;
use crate::state::AppState;
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
) -> Result<String, String> {
    let terminal_id = Uuid::new_v4().to_string();

    let tx = {
        let app_state = state.lock().await;
        app_state
            .get_connection_sender(&conn_id)
            .ok_or("Connection not found")?
    };

    let (respond_to, rx) = oneshot::channel();
    tx.send(ConnectionRequest::CreatePty {
        terminal_id: terminal_id.clone(),
        working_dir,
        respond_to,
    })
    .await
    .map_err(|_| "Connection is closed".to_string())?;

    let pty_session = rx
        .await
        .map_err(|_| "Connection is closed".to_string())?
        .map_err(|e| e.to_string())?;

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
) -> Result<(), String> {
    let mut app_state = state.lock().await;

    let terminal = app_state
        .get_terminal_mut(&term_id)
        .ok_or("Terminal not found")?;

    terminal.write(&data).await.map_err(|e| e.to_string())?;

    Ok(())
}

/// Resize a terminal
#[tauri::command]
pub async fn terminal_resize(
    state: State<'_, Arc<Mutex<AppState>>>,
    term_id: String,
    cols: u32,
    rows: u32,
) -> Result<(), String> {
    let mut app_state = state.lock().await;

    let terminal = app_state
        .get_terminal_mut(&term_id)
        .ok_or("Terminal not found")?;

    terminal
        .resize(cols, rows)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Close a terminal session
#[tauri::command]
pub async fn terminal_close(
    state: State<'_, Arc<Mutex<AppState>>>,
    term_id: String,
) -> Result<(), String> {
    let mut app_state = state.lock().await;

    if let Some(mut terminal) = app_state.remove_terminal(&term_id) {
        terminal.close().await.map_err(|e| e.to_string())?;
        log::info!("Terminal session closed: {}", term_id);
    }

    Ok(())
}
