use crate::state::AppState;
use crate::ssh::runtime;
use std::sync::Arc;
use tauri::{AppHandle, State};
use tokio::sync::Mutex;
use uuid::Uuid;

/// Create a new terminal session
#[tauri::command]
pub async fn terminal_create(
    app: AppHandle,
    state: State<'_, Arc<Mutex<AppState>>>,
    conn_id: String,
    working_dir: Option<String>,
) -> Result<String, String> {
    let state = state.inner().clone();

    runtime::spawn(async move {
    let terminal_id = Uuid::new_v4().to_string();

    let mut app_state = state.lock().await;

    let connection = app_state
        .get_connection_mut(&conn_id)
        .ok_or("Connection not found")?;

    let pty_session = connection
        .create_pty_session(terminal_id.clone(), conn_id.clone(), app.clone(), working_dir)
        .await
        .map_err(|e| e.to_string())?;

    app_state.add_terminal(terminal_id.clone(), pty_session);

    log::info!("Terminal session created: {}", terminal_id);

    Ok(terminal_id)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Write data to a terminal
#[tauri::command]
pub async fn terminal_write(
    state: State<'_, Arc<Mutex<AppState>>>,
    term_id: String,
    data: Vec<u8>,
) -> Result<(), String> {
    let state = state.inner().clone();

    runtime::spawn(async move {
        let mut app_state = state.lock().await;

        let terminal = app_state
            .get_terminal_mut(&term_id)
            .ok_or("Terminal not found")?;

        terminal.write(&data).await.map_err(|e| e.to_string())?;

        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Resize a terminal
#[tauri::command]
pub async fn terminal_resize(
    state: State<'_, Arc<Mutex<AppState>>>,
    term_id: String,
    cols: u32,
    rows: u32,
) -> Result<(), String> {
    let state = state.inner().clone();

    runtime::spawn(async move {
        let mut app_state = state.lock().await;

        let terminal = app_state
            .get_terminal_mut(&term_id)
            .ok_or("Terminal not found")?;

        terminal
            .resize(cols, rows)
            .await
            .map_err(|e| e.to_string())?;

        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Close a terminal session
#[tauri::command]
pub async fn terminal_close(
    state: State<'_, Arc<Mutex<AppState>>>,
    term_id: String,
) -> Result<(), String> {
    let state = state.inner().clone();

    runtime::spawn(async move {
        let mut app_state = state.lock().await;

        if let Some(mut terminal) = app_state.remove_terminal(&term_id) {
            terminal.close().await.map_err(|e| e.to_string())?;
            log::info!("Terminal session closed: {}", term_id);
        }

        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}
