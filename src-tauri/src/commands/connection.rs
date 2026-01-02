#![allow(dead_code)]
use crate::ipc_error::IpcError;
use crate::ssh::auth::AuthMethod;
use crate::ssh::actor::{spawn_connection_actor, ConnectionRequest};
use crate::ssh::client::{SshConnection, SshError};
use crate::state::AppState;
use crate::trace::{emit_trace, TraceEvent};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tauri::{AppHandle, State};
use tokio::sync::Mutex;
use tokio::sync::oneshot;
use tokio::time::{timeout, Duration, sleep};
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

fn map_connect_error(profile: &ConnectionProfile, error: SshError) -> IpcError {
    let base_context = json!({
        "host": profile.host,
        "port": profile.port,
        "username": profile.username,
        "authMethod": profile.auth_method,
    });

    match error {
        SshError::DnsLookupFailed { host, port, detail } => IpcError::new(
            "dns_lookup_failed",
            "DNS lookup failed. Check the hostname and network connectivity.",
        )
        .with_raw(detail)
        .with_context(json!({ "host": host, "port": port, "profile": base_context })),
        SshError::TcpConnectFailed { addr, detail } => IpcError::new(
            "tcp_connect_failed",
            "TCP connection failed. Check the address, port, and firewall rules.",
        )
        .with_raw(detail)
        .with_context(json!({ "addr": addr.to_string(), "profile": base_context })),
        SshError::TcpConnectTimeout { addr } => IpcError::new(
            "tcp_connect_timeout",
            "TCP connection timed out. This is often caused by a blocked port or unstable network.",
        )
        .with_context(json!({ "addr": addr.to_string(), "profile": base_context })),
        SshError::HandshakeJoinAborted { addr } => IpcError::new(
            "ssh_handshake_aborted",
            "SSH handshake aborted (JoinError). This often indicates a network drop or the server closing the connection early.",
        )
        .with_context(json!({ "addr": addr.to_string(), "profile": base_context })),
        SshError::HandshakeFailed { addr, detail } => IpcError::new(
            "ssh_handshake_failed",
            "SSH handshake failed. Verify server compatibility and network stability.",
        )
        .with_raw(detail)
        .with_context(json!({ "addr": addr.to_string(), "profile": base_context })),
        SshError::AuthenticationFailed(source) => IpcError::new(
            "ssh_auth_failed",
            "SSH authentication failed. Verify username and credentials.",
        )
        .with_raw(source)
        .with_context(json!({ "profile": base_context })),
        other => IpcError::new("ssh_connect_failed", "SSH connection failed")
            .with_raw(other.to_string())
            .with_context(json!({ "profile": base_context })),
    }
}

/// Connect to a remote machine via SSH
#[tauri::command]
pub async fn ssh_connect(
    app: AppHandle,
    state: State<'_, Arc<Mutex<AppState>>>,
    profile: ConnectionProfile,
    password: Option<String>,
) -> Result<String, IpcError> {
    let auth = match profile.auth_method.as_str() {
        "key" => {
            let key_path = profile
                .key_path
                .clone()
                .ok_or_else(|| IpcError::new("invalid_key_path", "Key path required for key authentication"))?;
            AuthMethod::Key {
                path: key_path,
                passphrase: password,
            }
        }
        "password" => AuthMethod::Password(
            password.ok_or_else(|| {
                IpcError::new("missing_password", "Password required for password authentication")
            })?,
        ),
        _ => return Err(IpcError::new("invalid_auth_method", "Invalid authentication method")),
    };

    let mut connection = SshConnection::connect(
        &profile.host,
        profile.port,
        &profile.username,
        auth,
        Some(&app),
    )
    .await
    .map_err(|e| map_connect_error(&profile, e))?;

    // DriftCode requires SFTP for file browsing/editing; fail fast with a clear message
    // if the server does not support the SFTP subsystem.
    emit_trace(&app, TraceEvent::new("sftp", "verify", "Verifying SFTP availability"));
    if let Err(e) = connection.get_home_dir().await {
        emit_trace(&app, TraceEvent::new("sftp", "failed", "SFTP unavailable on server").with_detail(e.to_string()).error());
        let _ = connection.disconnect().await;
        return Err(
            IpcError::new(
                "sftp_unavailable",
                "Connected, but SFTP is unavailable on this server.",
            )
            .with_raw(e.to_string())
            .with_context(json!({
                "host": profile.host,
                "port": profile.port,
                "username": profile.username,
            })),
        );
    }
    emit_trace(&app, TraceEvent::new("sftp", "ok", "SFTP subsystem available"));

    let connection_id = Uuid::new_v4().to_string();
    emit_trace(&app, TraceEvent::new("actor", "spawn", "Spawning connection actor").with_detail(&connection_id));

    let mut app_state = state.lock().await;
    let handle = spawn_connection_actor(app.clone(), connection_id.clone(), connection);
    app_state.add_connection(connection_id.clone(), handle);

    emit_trace(&app, TraceEvent::new("connect", "complete", &format!("Connection ready: {}", connection_id)));
    log::info!("SSH connection established: {}", connection_id);

    Ok(connection_id)
}

/// Disconnect from a remote machine
#[tauri::command]
pub async fn ssh_disconnect(
    state: State<'_, Arc<Mutex<AppState>>>,
    conn_id: String,
) -> Result<(), IpcError> {
    let handle = {
        let mut app_state = state.lock().await;
        app_state.remove_connection(&conn_id)
    };

    if let Some(handle) = handle {
        let (respond_to, rx) = oneshot::channel();
        let _ = handle
            .tx
            .send(ConnectionRequest::Disconnect { respond_to })
            .await;

        match timeout(Duration::from_secs(5), rx).await {
            Ok(Ok(Ok(()))) => {}
            Ok(Ok(Err(e))) => {
                log::warn!("SSH disconnect error for {}: {}", conn_id, e);
            }
            Ok(Err(_)) | Err(_) => {
                // Actor is unresponsive; abort to avoid leaking tasks.
                handle.task.abort();
            }
        }

        log::info!("SSH connection closed: {}", conn_id);
    }

    Ok(())
}

/// Get the home directory for the current connection
#[tauri::command]
pub async fn ssh_get_home_dir(
    state: State<'_, Arc<Mutex<AppState>>>,
    conn_id: String,
) -> Result<String, IpcError> {
    let tx = {
        let app_state = state.lock().await;
        app_state
            .get_connection_sender(&conn_id)
            .ok_or_else(|| IpcError::new("connection_not_found", "Connection not found"))?
    };

    let (respond_to, rx) = oneshot::channel();
    tx.send(ConnectionRequest::GetHomeDir { respond_to })
        .await
        .map_err(|_| IpcError::new("connection_closed", "Connection is closed"))?;

    rx.await
        .map_err(|_| IpcError::new("connection_closed", "Connection is closed"))?
        .map_err(|e| IpcError::new("ssh_home_dir_failed", "Failed to get home directory").with_raw(e.to_string()))
}

/// Test a connection without persisting it
#[tauri::command]
pub async fn ssh_test_connection(
    app: AppHandle,
    profile: ConnectionProfile,
    password: Option<String>,
) -> Result<bool, IpcError> {
    let auth = match profile.auth_method.as_str() {
        "key" => {
            let key_path = profile
                .key_path
                .clone()
                .ok_or_else(|| IpcError::new("invalid_key_path", "Key path required for key authentication"))?;
            AuthMethod::Key {
                path: key_path,
                passphrase: password,
            }
        }
        "password" => AuthMethod::Password(
            password.ok_or_else(|| {
                IpcError::new("missing_password", "Password required for password authentication")
            })?,
        ),
        _ => return Err(IpcError::new("invalid_auth_method", "Invalid authentication method")),
    };

    emit_trace(&app, TraceEvent::new("test", "start", &format!("Testing connection to {}:{}", profile.host, profile.port)));

    match SshConnection::connect(&profile.host, profile.port, &profile.username, auth, Some(&app)).await {
        Ok(mut conn) => {
            emit_trace(&app, TraceEvent::new("sftp", "verify", "Verifying SFTP availability (test)"));
            if let Err(e) = conn.get_home_dir().await {
                emit_trace(&app, TraceEvent::new("sftp", "failed", "SFTP unavailable").with_detail(e.to_string()).error());
                let _ = conn.disconnect().await;
                return Err(
                    IpcError::new(
                        "sftp_unavailable",
                        "SFTP is unavailable on this server. Enable the SSH SFTP subsystem and try again.",
                    )
                    .with_raw(e.to_string())
                    .with_context(json!({
                        "host": profile.host,
                        "port": profile.port,
                        "username": profile.username,
                    })),
                );
            }
            emit_trace(&app, TraceEvent::new("sftp", "ok", "SFTP subsystem available"));

            emit_trace(&app, TraceEvent::new("test", "disconnect", "Test complete, disconnecting"));
            let _ = conn.disconnect().await;
            // Grace period for TCP socket release - prevents "handshake aborted" when
            // connect is called immediately after test on LAN/WiFi networks.
            emit_trace(&app, TraceEvent::new("test", "grace_period", "Waiting 150ms for socket release"));
            sleep(Duration::from_millis(150)).await;
            emit_trace(&app, TraceEvent::new("test", "success", "Connection test passed"));
            Ok(true)
        }
        Err(e) => {
            emit_trace(&app, TraceEvent::new("test", "failed", "Connection test failed").with_detail(e.to_string()).error());
            Err(map_connect_error(&profile, e))
        }
    }
}
