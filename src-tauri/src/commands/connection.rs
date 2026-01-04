#![allow(dead_code)]
use crate::ipc_error::IpcError;
use crate::ssh::auth::AuthMethod;
use crate::ssh::actor::{spawn_connection_actor, ConnectionRequest};
use crate::ssh::client::{SshConnection, SshError};
use crate::ssh::known_hosts;
use crate::state::AppState;
use crate::trace::{emit_trace, TraceEvent};
use serde::{Deserialize, Serialize};
use serde_json::json;
use ssh_key::HashAlg;
use ssh_key::PublicKey as SshPublicKey;
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
        SshError::HandshakeJoinAborted { addr, detail, diag } => IpcError::new(
            "ssh_handshake_aborted",
            "SSH handshake aborted (JoinError). This often indicates a network drop or the server closing the connection early.",
        )
        .with_raw(detail.unwrap_or_else(|| "JoinError".to_string()))
        .with_context(json!({
            "addr": addr.to_string(),
            "profile": base_context,
            "handshake": diag,
        })),
        SshError::HandshakeFailed { addr, detail, diag } => IpcError::new(
            "ssh_handshake_failed",
            "SSH handshake failed. Verify server compatibility and network stability.",
        )
        .with_raw(detail)
        .with_context(json!({
            "addr": addr.to_string(),
            "profile": base_context,
            "handshake": diag,
        })),
        SshError::HostKeyUntrusted {
            host,
            port,
            key_type,
            fingerprint_sha256,
            public_key_openssh,
        } => IpcError::new(
            "ssh_hostkey_untrusted",
            "The server's host key is not trusted yet.",
        )
        .with_context(json!({
            "host": host,
            "port": port,
            "keyType": key_type,
            "fingerprintSha256": fingerprint_sha256,
            "publicKeyOpenssh": public_key_openssh,
            "profile": base_context,
        })),
        SshError::HostKeyMismatch {
            host,
            port,
            key_type,
            expected_fingerprint_sha256,
            actual_fingerprint_sha256,
            expected_public_key_openssh,
            actual_public_key_openssh,
        } => IpcError::new(
            "ssh_hostkey_mismatch",
            "The server's host key has changed (possible MITM).",
        )
        .with_context(json!({
            "host": host,
            "port": port,
            "keyType": key_type,
            "expectedFingerprintSha256": expected_fingerprint_sha256,
            "actualFingerprintSha256": actual_fingerprint_sha256,
            "expectedPublicKeyOpenssh": expected_public_key_openssh,
            "actualPublicKeyOpenssh": actual_public_key_openssh,
            "profile": base_context,
        })),
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
        &app,
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

/// Reconnect an existing connection ID (keeps the same connId so the UI can recover sessions).
#[tauri::command]
pub async fn ssh_reconnect(
    app: AppHandle,
    state: State<'_, Arc<Mutex<AppState>>>,
    conn_id: String,
    profile: ConnectionProfile,
    password: Option<String>,
) -> Result<(), IpcError> {
    // Best-effort: remove any existing handle for this connection ID (stale or active).
    // Also drop any existing PTY sessions for this connection; the UI will re-open them after reconnect.
    let stale_terminals = {
        let mut app_state = state.lock().await;
        let terminals = app_state.take_terminals_for_connection(&conn_id);
        if let Some(handle) = app_state.remove_connection(&conn_id) {
            handle.task.abort();
        }
        terminals
    };
    for mut terminal in stale_terminals {
        // Best-effort cleanup; avoid blocking reconnect if this hangs.
        let _ = timeout(Duration::from_millis(500), terminal.close()).await;
    }

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

    emit_trace(
        &app,
        TraceEvent::new("ssh", "reconnect", &format!("Reconnecting: {}", conn_id))
            .with_detail(format!("{}@{}:{}", profile.username, profile.host, profile.port)),
    );

    let mut connection = SshConnection::connect(
        &profile.host,
        profile.port,
        &profile.username,
        auth,
        &app,
    )
    .await
    .map_err(|e| map_connect_error(&profile, e))?;

    // Ensure SFTP is available (same requirement as initial connect).
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

    emit_trace(&app, TraceEvent::new("actor", "spawn", "Spawning connection actor").with_detail(&conn_id));
    let handle = spawn_connection_actor(app.clone(), conn_id.clone(), connection);

    let mut app_state = state.lock().await;
    app_state.add_connection(conn_id.clone(), handle);

    emit_trace(&app, TraceEvent::new("connect", "complete", &format!("Connection ready: {}", conn_id)));
    Ok(())
}

/// Disconnect from a remote machine
#[tauri::command]
pub async fn ssh_disconnect(
    state: State<'_, Arc<Mutex<AppState>>>,
    conn_id: String,
) -> Result<(), IpcError> {
    let (handle, terminals) = {
        let mut app_state = state.lock().await;
        let handle = app_state.remove_connection(&conn_id);
        let terminals = app_state.take_terminals_for_connection(&conn_id);
        (handle, terminals)
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

    // Close any PTY sessions that were using this connection.
    for mut terminal in terminals {
        let _ = timeout(Duration::from_millis(500), terminal.close()).await;
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

/// Check whether `tmux` is available on the server for an active connection.
#[tauri::command]
pub async fn ssh_check_tmux(
    state: State<'_, Arc<Mutex<AppState>>>,
    conn_id: String,
) -> Result<bool, IpcError> {
    let tx = {
        let app_state = state.lock().await;
        app_state
            .get_connection_sender(&conn_id)
            .ok_or_else(|| IpcError::new("connection_not_found", "Connection not found"))?
    };

    let (respond_to, rx) = oneshot::channel();
    tx.send(ConnectionRequest::CheckTmux { respond_to })
        .await
        .map_err(|_| IpcError::new("connection_closed", "Connection is closed"))?;

    timeout(Duration::from_secs(6), rx)
        .await
        .map_err(|_| IpcError::new("tmux_check_timeout", "tmux check timed out"))?
        .map_err(|_| IpcError::new("connection_closed", "Connection is closed"))?
        .map_err(|e| IpcError::new("tmux_check_failed", "Failed to check tmux availability").with_raw(e.to_string()))
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

    match SshConnection::connect(&profile.host, profile.port, &profile.username, auth, &app).await {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustHostKeyRequest {
    pub host: String,
    pub port: u16,
    pub key_type: String,
    pub fingerprint_sha256: String,
    pub public_key_openssh: String,
}

/// List all trusted host keys.
#[tauri::command]
pub async fn ssh_list_trusted_host_keys(
    app: AppHandle,
) -> Result<Vec<known_hosts::KnownHostEntry>, IpcError> {
    known_hosts::list(&app)
        .await
        .map_err(|e| IpcError::new("hostkey_store_failed", "Failed to read trusted host keys").with_raw(e))
}

/// Persist a trusted host key for `host:port`.
#[tauri::command]
pub async fn ssh_trust_host_key(app: AppHandle, request: TrustHostKeyRequest) -> Result<(), IpcError> {
    let parsed = SshPublicKey::from_openssh(&request.public_key_openssh).map_err(|e| {
        IpcError::new("invalid_public_key", "Invalid public key format").with_raw(e.to_string())
    })?;

    let computed = parsed.fingerprint(HashAlg::Sha256).to_string();
    if computed != request.fingerprint_sha256 {
        return Err(
            IpcError::new("hostkey_fingerprint_mismatch", "Fingerprint does not match provided public key")
                .with_context(json!({
                    "computed": computed,
                    "provided": request.fingerprint_sha256,
                    "host": request.host,
                    "port": request.port,
                })),
        );
    }

    known_hosts::upsert(
        &app,
        &request.host,
        request.port,
        &request.key_type,
        &request.fingerprint_sha256,
        &request.public_key_openssh,
    )
    .await
    .map_err(|e| IpcError::new("hostkey_store_failed", "Failed to save trusted host key").with_raw(e))?;

    emit_trace(
        &app,
        TraceEvent::new("hostkey", "trusted_saved", "Trusted host key saved")
            .with_detail(format!("{}:{} {}", request.host, request.port, request.fingerprint_sha256)),
    );

    Ok(())
}

/// Forget a previously trusted host key for `host:port`.
#[tauri::command]
pub async fn ssh_forget_host_key(app: AppHandle, host: String, port: u16) -> Result<(), IpcError> {
    known_hosts::remove(&app, &host, port)
        .await
        .map_err(|e| IpcError::new("hostkey_store_failed", "Failed to remove trusted host key").with_raw(e))?;
    emit_trace(
        &app,
        TraceEvent::new("hostkey", "trusted_removed", "Trusted host key removed")
            .with_detail(format!("{}:{}", host, port)),
    );
    Ok(())
}
