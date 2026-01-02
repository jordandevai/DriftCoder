#![allow(dead_code)]
use crate::ssh::auth::AuthMethod;
use crate::ssh::client::SshConnection;
use crate::ssh::runtime;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionProfile {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth_method: String,
    pub key_path: Option<String>,
}

#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct ConnectionError {
    message: String,
}

impl From<String> for ConnectionError {
    fn from(message: String) -> Self {
        Self { message }
    }
}

impl From<&str> for ConnectionError {
    fn from(message: &str) -> Self {
        Self {
            message: message.to_string(),
        }
    }
}

/// Connect to a remote machine via SSH
#[tauri::command]
pub async fn ssh_connect(
    state: State<'_, Arc<Mutex<AppState>>>,
    profile: ConnectionProfile,
    password: Option<String>,
) -> Result<String, String> {
    let state = state.inner().clone();

    runtime::spawn(async move {
    let auth = match profile.auth_method.as_str() {
        "key" => {
            let key_path = profile
                .key_path
                .clone()
                .ok_or("Key path required for key authentication")?;
            AuthMethod::Key {
                path: key_path,
                passphrase: password,
            }
        }
        "password" => AuthMethod::Password(
            password.ok_or("Password required for password authentication")?,
        ),
        _ => return Err("Invalid authentication method".to_string()),
    };

    let mut connection = SshConnection::connect(
        &profile.host,
        profile.port,
        &profile.username,
        auth,
    )
    .await
    .map_err(|e| e.to_string())?;

    // DriftCode requires SFTP for file browsing/editing; fail fast with a clear message
    // if the server does not support the SFTP subsystem.
    if let Err(e) = connection.get_home_dir().await {
        let _ = connection.disconnect().await;
        return Err(format!(
            "Connected, but SFTP is unavailable on this server. Enable the SSH SFTP subsystem and try again. Details: {}",
            e
        ));
    }

    let connection_id = Uuid::new_v4().to_string();

    let mut app_state = state.lock().await;
    app_state.add_connection(connection_id.clone(), connection);

    log::info!("SSH connection established: {}", connection_id);

    Ok(connection_id)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Disconnect from a remote machine
#[tauri::command]
pub async fn ssh_disconnect(
    state: State<'_, Arc<Mutex<AppState>>>,
    conn_id: String,
) -> Result<(), String> {
    let state = state.inner().clone();

    runtime::spawn(async move {
    let mut app_state = state.lock().await;

    if let Some(mut connection) = app_state.remove_connection(&conn_id) {
        connection.disconnect().await.map_err(|e| e.to_string())?;
        log::info!("SSH connection closed: {}", conn_id);
    }

    Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Get the home directory for the current connection
#[tauri::command]
pub async fn ssh_get_home_dir(
    state: State<'_, Arc<Mutex<AppState>>>,
    conn_id: String,
) -> Result<String, String> {
    let state = state.inner().clone();

    runtime::spawn(async move {
    let mut app_state = state.lock().await;

    let connection = app_state
        .get_connection_mut(&conn_id)
        .ok_or("Connection not found")?;

    connection.get_home_dir().await.map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Test a connection without persisting it
#[tauri::command]
pub async fn ssh_test_connection(
    profile: ConnectionProfile,
    password: Option<String>,
) -> Result<bool, String> {
    runtime::spawn(async move {
    let auth = match profile.auth_method.as_str() {
        "key" => {
            let key_path = profile
                .key_path
                .clone()
                .ok_or("Key path required for key authentication")?;
            AuthMethod::Key {
                path: key_path,
                passphrase: password,
            }
        }
        "password" => AuthMethod::Password(
            password.ok_or("Password required for password authentication")?,
        ),
        _ => return Err("Invalid authentication method".to_string()),
    };

    match SshConnection::connect(&profile.host, profile.port, &profile.username, auth).await {
        Ok(mut conn) => {
            if let Err(e) = conn.get_home_dir().await {
                let _ = conn.disconnect().await;
                return Err(format!(
                    "SFTP is unavailable on this server. Enable the SSH SFTP subsystem and try again. Details: {}",
                    e
                ));
            }

            let _ = conn.disconnect().await;
            Ok(true)
        }
        Err(e) => {
            log::warn!("Connection test failed: {}", e);
            Err(e.to_string())
        }
    }
    })
    .await
    .map_err(|e| e.to_string())?
}
