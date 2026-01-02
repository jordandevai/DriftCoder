use crate::diagnostics;
use crate::ssh::auth::AuthMethod;
use crate::ssh::known_hosts;
use crate::ssh::pty::PtySession;
use crate::ssh::sftp::{SftpEntry, SftpStat};
use crate::trace::{emit_trace, TraceEvent};
use async_trait::async_trait;
use russh::client::{self, Config, Handle, Handler};
use russh::Disconnect;
use russh_sftp::client::error::Error as SftpClientError;
use russh_sftp::client::SftpSession;
use serde::Serialize;
use ssh_key::public::PublicKey;
use ssh_key::HashAlg;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tauri::AppHandle;
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};
use tokio::net::{lookup_host, TcpStream};
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HandshakeDiag {
    pub attempt_id: String,
    pub client_id: Option<String>,
    pub server_id: Option<String>,
    pub bytes_written: u64,
    pub bytes_read: u64,
}

#[derive(Default)]
struct HandshakeTranscript {
    bytes_read: AtomicU64,
    bytes_written: AtomicU64,
    client_buf: StdMutex<Vec<u8>>,
    server_buf: StdMutex<Vec<u8>>,
    client_id: StdMutex<Option<String>>,
    server_id: StdMutex<Option<String>>,
}

impl HandshakeTranscript {
    fn on_write(&self, data: &[u8]) {
        self.bytes_written
            .fetch_add(data.len() as u64, Ordering::Relaxed);

        if self.client_id.lock().map(|v| v.is_some()).unwrap_or(false) {
            return;
        }

        let mut buf = self.client_buf.lock().unwrap_or_else(|e| e.into_inner());
        if buf.len() < 2048 {
            let remaining = 2048usize.saturating_sub(buf.len());
            buf.extend_from_slice(&data[..data.len().min(remaining)]);
        }

        if let Some(pos) = buf.iter().position(|b| *b == b'\n') {
            let line = &buf[..=pos];
            let s = String::from_utf8_lossy(line).trim().to_string();
            let mut id = self.client_id.lock().unwrap_or_else(|e| e.into_inner());
            if id.is_none() && s.starts_with("SSH-") {
                *id = Some(s);
            }
        }
    }

    fn on_read(&self, data: &[u8]) {
        self.bytes_read
            .fetch_add(data.len() as u64, Ordering::Relaxed);

        if self.server_id.lock().map(|v| v.is_some()).unwrap_or(false) {
            return;
        }

        let mut buf = self.server_buf.lock().unwrap_or_else(|e| e.into_inner());
        if buf.len() < 2048 {
            let remaining = 2048usize.saturating_sub(buf.len());
            buf.extend_from_slice(&data[..data.len().min(remaining)]);
        }

        if let Some(pos) = buf.iter().position(|b| *b == b'\n') {
            let line = &buf[..=pos];
            let s = String::from_utf8_lossy(line).trim().to_string();
            let mut id = self.server_id.lock().unwrap_or_else(|e| e.into_inner());
            if id.is_none() && s.starts_with("SSH-") {
                *id = Some(s);
            }
        }
    }

    fn snapshot(&self, attempt_id: &str) -> HandshakeDiag {
        let client_id = self.client_id.lock().ok().and_then(|v| v.clone());
        let server_id = self.server_id.lock().ok().and_then(|v| v.clone());
        HandshakeDiag {
            attempt_id: attempt_id.to_string(),
            client_id,
            server_id,
            bytes_written: self.bytes_written.load(Ordering::Relaxed),
            bytes_read: self.bytes_read.load(Ordering::Relaxed),
        }
    }
}

struct InstrumentedTcpStream {
    inner: TcpStream,
    transcript: Arc<HandshakeTranscript>,
}

impl InstrumentedTcpStream {
    fn new(inner: TcpStream, transcript: Arc<HandshakeTranscript>) -> Self {
        Self { inner, transcript }
    }
}

impl AsyncRead for InstrumentedTcpStream {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let before = buf.filled().len();
        let poll = std::pin::Pin::new(&mut self.inner).poll_read(cx, buf);
        if let std::task::Poll::Ready(Ok(())) = &poll {
            let after = buf.filled().len();
            if after > before {
                self.transcript.on_read(&buf.filled()[before..after]);
            }
        }
        poll
    }
}

impl AsyncWrite for InstrumentedTcpStream {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        data: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        let poll = std::pin::Pin::new(&mut self.inner).poll_write(cx, data);
        if let std::task::Poll::Ready(Ok(n)) = &poll {
            if *n > 0 {
                self.transcript.on_write(&data[..*n]);
            }
        }
        poll
    }

    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::pin::Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::pin::Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

#[derive(Debug, Error)]
pub enum SshError {
    #[error("DNS lookup failed for {host}:{port}: {detail}")]
    DnsLookupFailed {
        host: String,
        port: u16,
        detail: String,
    },
    #[error("TCP connect to {addr} failed: {detail}")]
    TcpConnectFailed { addr: SocketAddr, detail: String },
    #[error("TCP connect to {addr} timed out")]
    TcpConnectTimeout { addr: SocketAddr },
    #[error("SSH handshake to {addr} failed: {detail}")]
    HandshakeFailed {
        addr: SocketAddr,
        detail: String,
        #[allow(dead_code)]
        diag: Option<HandshakeDiag>,
    },
    #[error("SSH handshake to {addr} aborted (JoinError)")]
    HandshakeJoinAborted {
        addr: SocketAddr,
        #[allow(dead_code)]
        detail: Option<String>,
        #[allow(dead_code)]
        diag: Option<HandshakeDiag>,
    },
    #[error("Untrusted host key for {host}:{port} ({fingerprint_sha256})")]
    HostKeyUntrusted {
        host: String,
        port: u16,
        key_type: String,
        fingerprint_sha256: String,
        public_key_openssh: String,
    },
    #[error("Host key mismatch for {host}:{port} (expected {expected_fingerprint_sha256}, got {actual_fingerprint_sha256})")]
    HostKeyMismatch {
        host: String,
        port: u16,
        key_type: String,
        expected_fingerprint_sha256: String,
        actual_fingerprint_sha256: String,
        expected_public_key_openssh: String,
        actual_public_key_openssh: String,
    },
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),
    #[error("SFTP error: {0}")]
    SftpError(String),
    #[error("SFTP request timed out")]
    SftpTimeout,
    #[error("SFTP session closed")]
    SftpSessionClosed,
    #[error("Channel error: {0}")]
    ChannelError(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// SSH client handler
#[derive(Clone)]
struct ClientHandler {
    app: AppHandle,
    host: String,
    port: u16,
    correlation_id: String,
}

#[derive(Debug, Error)]
enum ClientError {
    #[error(transparent)]
    Russh(#[from] russh::Error),
    #[error("Host key store error: {0}")]
    HostKeyStore(String),
    #[error("Host key untrusted: {host}:{port} {fingerprint_sha256}")]
    HostKeyUntrusted {
        host: String,
        port: u16,
        key_type: String,
        fingerprint_sha256: String,
        public_key_openssh: String,
    },
    #[error("Host key mismatch: {host}:{port} expected={expected_fingerprint_sha256} got={actual_fingerprint_sha256}")]
    HostKeyMismatch {
        host: String,
        port: u16,
        key_type: String,
        expected_fingerprint_sha256: String,
        actual_fingerprint_sha256: String,
        expected_public_key_openssh: String,
        actual_public_key_openssh: String,
    },
}

#[async_trait]
impl Handler for ClientHandler {
    type Error = ClientError;

    async fn check_server_key(
        &mut self,
        server_public_key: &PublicKey,
    ) -> Result<bool, Self::Error> {
        let key_type = server_public_key.algorithm().as_str().to_string();
        let fingerprint = server_public_key
            .fingerprint(HashAlg::Sha256)
            .to_string();
        let public_key_openssh = server_public_key
            .to_openssh()
            .unwrap_or_else(|_| "<failed to encode public key>".to_string());

        emit_trace(
            &self.app,
            TraceEvent::new("hostkey", "check", "Checking server host key")
                .with_correlation_id(self.correlation_id.clone())
                .with_detail(format!("{}:{} {}", self.host, self.port, fingerprint)),
        );

        let existing = known_hosts::get(&self.app, &self.host, self.port)
            .await
            .map_err(ClientError::HostKeyStore)?;

        match existing {
            None => Err(ClientError::HostKeyUntrusted {
                host: self.host.clone(),
                port: self.port,
                key_type,
                fingerprint_sha256: fingerprint,
                public_key_openssh,
            }),
            Some(entry) if entry.fingerprint_sha256 == fingerprint => {
                emit_trace(
                    &self.app,
                    TraceEvent::new("hostkey", "trusted", "Host key trusted")
                        .with_correlation_id(self.correlation_id.clone()),
                );
                Ok(true)
            }
            Some(entry) => Err(ClientError::HostKeyMismatch {
                host: self.host.clone(),
                port: self.port,
                key_type,
                expected_fingerprint_sha256: entry.fingerprint_sha256,
                actual_fingerprint_sha256: fingerprint,
                expected_public_key_openssh: entry.public_key_openssh,
                actual_public_key_openssh: public_key_openssh,
            }),
        }
    }
}

/// Represents an active SSH connection
pub struct SshConnection {
    handle: Handle<ClientHandler>,
    sftp: Option<Arc<Mutex<SftpSession>>>,
    #[allow(dead_code)]
    username: String,
}

impl SshConnection {
    pub fn reset_sftp(&mut self) {
        self.sftp = None;
    }

    /// Establish a new SSH connection
    ///
    /// If `app` is provided, trace events will be emitted for debugging.
    pub async fn connect(
        host: &str,
        port: u16,
        username: &str,
        auth: AuthMethod,
        app: &AppHandle,
    ) -> Result<Self, SshError> {
        let host = host.trim();
        let username = username.trim();

        // Helper to emit trace events
        let trace = |category: &str, step: &str, msg: &str, detail: Option<&str>, is_error: bool| {
            let mut event = TraceEvent::new(category, step, msg);
            if let Some(d) = detail {
                event = event.with_detail(d);
            }
            if is_error {
                event = event.error();
            }
            emit_trace(app, event);
        };

        trace("ssh", "start", &format!("Connecting to {}:{} as {}", host, port, username), None, false);

        let mut config = Config::default();
        // Mobile networks and some SFTP servers can be slow to respond; keepalives help
        // prevent idle connections from being dropped while the UI is loading.
        config.keepalive_interval = Some(Duration::from_secs(20));
        config.keepalive_max = 3;
        let config = Arc::new(config);

        trace("dns", "lookup", &format!("Resolving {}:{}", host, port), None, false);

        let mut resolved: Vec<std::net::SocketAddr> = lookup_host((host, port))
            .await
            .map_err(|e| {
                trace("dns", "failed", "DNS lookup failed", Some(&e.to_string()), true);
                SshError::DnsLookupFailed {
                    host: host.to_string(),
                    port,
                    detail: e.to_string(),
                }
            })?
            .collect();

        if resolved.is_empty() {
            trace("dns", "failed", "DNS returned no addresses", None, true);
            return Err(SshError::ConnectionFailed(format!(
                "DNS lookup returned no addresses for {}:{}",
                host, port
            )));
        }

        let addr_list: Vec<String> = resolved.iter().map(|a| a.to_string()).collect();
        trace(
            "dns",
            "resolved",
            &format!("Found {} addresses", resolved.len()),
            Some(&addr_list.join(", ")),
            false,
        );

        // Prefer IPv4 to avoid IPv6-only / broken IPv6 routes on some networks.
        resolved.sort_by_key(|a| match a {
            std::net::SocketAddr::V4(_) => 0,
            std::net::SocketAddr::V6(_) => 1,
        });

        let mut last_error: Option<SshError> = None;
        let mut handle: Option<Handle<ClientHandler>> = None;

        for (addr_idx, addr) in resolved.iter().copied().enumerate() {
            trace(
                "tcp",
                "attempt",
                &format!("Trying address {}/{}", addr_idx + 1, resolved.len()),
                Some(&addr.to_string()),
                false,
            );

            // Retry loop for JoinError - common on rapid reconnection after test disconnect.
            // We retry once with a brief delay, re-establishing the TCP connection.
            for attempt in 0..2 {
                        let attempt_id = Uuid::new_v4().to_string();
                        let transcript = Arc::new(HandshakeTranscript::default());

                        let trace_attempt = |category: &str,
                                             step: &str,
                                             msg: &str,
                                             detail: Option<&str>,
                                             is_error: bool| {
                            let mut event =
                                TraceEvent::new(category, step, msg).with_correlation_id(&attempt_id);
                            if let Some(d) = detail {
                                event = event.with_detail(d);
                            }
                            if is_error {
                                event = event.error();
                            }
                            emit_trace(app, event);
                        };

                if attempt > 0 {
                    trace_attempt(
                        "ssh",
                        "retry",
                        "Retrying after JoinError",
                        Some("200ms delay"),
                        false,
                    );
                    // Brief delay before retry to allow socket release
                    tokio::time::sleep(Duration::from_millis(200)).await;
                }

                trace_attempt(
                    "tcp",
                    "connect",
                    &format!("TCP connecting to {}", addr),
                    Some("8s timeout"),
                    false,
                );

                let socket = match tokio::time::timeout(Duration::from_secs(8), TcpStream::connect(addr))
                    .await
                {
                    Ok(Ok(s)) => {
                        trace_attempt("tcp", "connected", &format!("TCP connected to {}", addr), None, false);
                        s
                    }
                    Ok(Err(e)) => {
                        trace_attempt(
                            "tcp",
                            "failed",
                            &format!("TCP connect failed: {}", addr),
                            Some(&e.to_string()),
                            true,
                        );
                        diagnostics::record_connect_attempt(diagnostics::ConnectAttemptRecord {
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis() as u64,
                            attempt_id,
                            host: host.to_string(),
                            port,
                            username: username.to_string(),
                            addr: Some(addr.to_string()),
                            resolved_addrs: addr_list.clone(),
                            client_id: None,
                            server_id: None,
                            bytes_written: 0,
                            bytes_read: 0,
                            outcome: "tcp_connect_failed".to_string(),
                            outcome_detail: Some(e.to_string()),
                        });
                        last_error = Some(SshError::TcpConnectFailed {
                            addr,
                            detail: e.to_string(),
                        });
                        break; // TCP failed, try next address
                    }
                    Err(_) => {
                        trace_attempt(
                            "tcp",
                            "timeout",
                            &format!("TCP connect timed out: {}", addr),
                            None,
                            true,
                        );
                        diagnostics::record_connect_attempt(diagnostics::ConnectAttemptRecord {
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis() as u64,
                            attempt_id,
                            host: host.to_string(),
                            port,
                            username: username.to_string(),
                            addr: Some(addr.to_string()),
                            resolved_addrs: addr_list.clone(),
                            client_id: None,
                            server_id: None,
                            bytes_written: 0,
                            bytes_read: 0,
                            outcome: "tcp_connect_timeout".to_string(),
                            outcome_detail: None,
                        });
                        last_error = Some(SshError::TcpConnectTimeout { addr });
                        break; // TCP timeout, try next address
                    }
                };

                let _ = socket.set_nodelay(true);

                trace_attempt(
                    "ssh",
                    "handshake",
                    "Starting SSH handshake",
                    Some(&addr.to_string()),
                    false,
                );

                let socket = InstrumentedTcpStream::new(socket, transcript.clone());

                let handler = ClientHandler {
                    app: app.clone(),
                    host: host.to_string(),
                    port,
                    correlation_id: attempt_id.clone(),
                };

                match client::connect_stream(config.clone(), socket, handler).await {
                    Ok(h) => {
                        let diag = transcript.snapshot(&attempt_id);
                        trace_attempt(
                            "ssh",
                            "handshake_ok",
                            "SSH handshake successful",
                            diag.server_id.as_deref(),
                            false,
                        );
                        diagnostics::record_connect_attempt(diagnostics::ConnectAttemptRecord {
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis() as u64,
                            attempt_id: attempt_id.clone(),
                            host: host.to_string(),
                            port,
                            username: username.to_string(),
                            addr: Some(addr.to_string()),
                            resolved_addrs: addr_list.clone(),
                            client_id: diag.client_id.clone(),
                            server_id: diag.server_id.clone(),
                            bytes_written: diag.bytes_written,
                            bytes_read: diag.bytes_read,
                            outcome: "handshake_ok".to_string(),
                            outcome_detail: None,
                        });
                        handle = Some(h);
                        break;
                    }
                    Err(e) => {
                        let msg = e.to_string();
                        let diag = transcript.snapshot(&attempt_id);

                        match e {
                            ClientError::HostKeyStore(detail) => {
                                trace_attempt(
                                    "hostkey",
                                    "store_error",
                                    "Host key store error",
                                    Some(&detail),
                                    true,
                                );
                                last_error = Some(SshError::ConnectionFailed(detail));
                                break;
                            }
                            ClientError::HostKeyUntrusted {
                                host,
                                port,
                                key_type,
                                fingerprint_sha256,
                                public_key_openssh,
                            } => {
                                trace_attempt(
                                    "hostkey",
                                    "untrusted",
                                    "Host key untrusted",
                                    Some(&fingerprint_sha256),
                                    true,
                                );
                                last_error = Some(SshError::HostKeyUntrusted {
                                    host,
                                    port,
                                    key_type,
                                    fingerprint_sha256,
                                    public_key_openssh,
                                });
                                break;
                            }
                            ClientError::HostKeyMismatch {
                                host,
                                port,
                                key_type,
                                expected_fingerprint_sha256,
                                actual_fingerprint_sha256,
                                expected_public_key_openssh,
                                actual_public_key_openssh,
                            } => {
                                trace_attempt(
                                    "hostkey",
                                    "mismatch",
                                    "Host key mismatch",
                                    Some(&format!(
                                        "expected={} actual={}",
                                        expected_fingerprint_sha256, actual_fingerprint_sha256
                                    )),
                                    true,
                                );
                                last_error = Some(SshError::HostKeyMismatch {
                                    host,
                                    port,
                                    key_type,
                                    expected_fingerprint_sha256,
                                    actual_fingerprint_sha256,
                                    expected_public_key_openssh,
                                    actual_public_key_openssh,
                                });
                                break;
                            }
                            ClientError::Russh(russh::Error::Join(_)) => {
                                let detail = format!(
                                    "attempt {}/2; err={}; server_id={}",
                                    attempt + 1,
                                    msg,
                                    diag.server_id
                                        .clone()
                                        .unwrap_or_else(|| "unknown".to_string())
                                );
                                trace_attempt(
                                    "ssh",
                                    "join_error",
                                    "SSH handshake JoinError (will retry)",
                                    Some(&detail),
                                    true,
                                );
                                diagnostics::record_connect_attempt(diagnostics::ConnectAttemptRecord {
                                    timestamp: std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_millis() as u64,
                                    attempt_id: attempt_id.clone(),
                                    host: host.to_string(),
                                    port,
                                    username: username.to_string(),
                                    addr: Some(addr.to_string()),
                                    resolved_addrs: addr_list.clone(),
                                    client_id: diag.client_id.clone(),
                                    server_id: diag.server_id.clone(),
                                    bytes_written: diag.bytes_written,
                                    bytes_read: diag.bytes_read,
                                    outcome: "handshake_join_error".to_string(),
                                    outcome_detail: Some(msg.clone()),
                                });
                                last_error = Some(SshError::HandshakeJoinAborted {
                                    addr,
                                    detail: Some(msg),
                                    diag: Some(diag),
                                });
                                continue;
                            }
                            ClientError::Russh(other) => {
                                let msg = other.to_string();
                                trace_attempt(
                                    "ssh",
                                    "handshake_failed",
                                    "SSH handshake failed",
                                    Some(&msg),
                                    true,
                                );
                                diagnostics::record_connect_attempt(diagnostics::ConnectAttemptRecord {
                                    timestamp: std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_millis() as u64,
                                    attempt_id: attempt_id.clone(),
                                    host: host.to_string(),
                                    port,
                                    username: username.to_string(),
                                    addr: Some(addr.to_string()),
                                    resolved_addrs: addr_list.clone(),
                                    client_id: diag.client_id.clone(),
                                    server_id: diag.server_id.clone(),
                                    bytes_written: diag.bytes_written,
                                    bytes_read: diag.bytes_read,
                                    outcome: "handshake_failed".to_string(),
                                    outcome_detail: Some(msg.clone()),
                                });
                                last_error = Some(SshError::HandshakeFailed {
                                    addr,
                                    detail: msg,
                                    diag: Some(diag),
                                });
                                break;
                            }
                        }
                    }
                }
            }

            if handle.is_some() {
                break; // Success, exit address loop
            }
        }

        let mut handle = handle.ok_or_else(|| {
            trace("ssh", "all_failed", "All connection attempts failed", None, true);
            last_error.unwrap_or_else(|| {
                SshError::ConnectionFailed("Failed to establish SSH connection".to_string())
            })
        })?;

        // Authenticate
        let auth_method_str = match &auth {
            AuthMethod::Password(_) => "password",
            AuthMethod::Key { .. } => "publickey",
        };
        trace("auth", "start", &format!("Authenticating as {} via {}", username, auth_method_str), None, false);

        let auth_result = match &auth {
            AuthMethod::Password(password) => {
                trace("auth", "password", "Sending password authentication", None, false);
                handle
                    .authenticate_password(username, password)
                    .await
                    .map_err(|e| {
                        trace("auth", "failed", "Password auth error", Some(&e.to_string()), true);
                        SshError::AuthenticationFailed(e.to_string())
                    })?
            }
            AuthMethod::Key { .. } => {
                trace("auth", "key_load", "Loading SSH key pair", None, false);
                let key = auth
                    .load_key_pair()
                    .await
                    .map_err(|e| {
                        trace("auth", "key_load_failed", "Failed to load key", Some(&e.to_string()), true);
                        SshError::AuthenticationFailed(e.to_string())
                    })?
                    .ok_or_else(|| {
                        trace("auth", "no_key", "No key pair loaded", None, true);
                        SshError::AuthenticationFailed("No key pair loaded".to_string())
                    })?;

                trace("auth", "publickey", "Sending public key authentication", None, false);
                handle
                    .authenticate_publickey(username, key)
                    .await
                    .map_err(|e| {
                        trace("auth", "failed", "Public key auth error", Some(&e.to_string()), true);
                        SshError::AuthenticationFailed(e.to_string())
                    })?
            }
        };

        if !auth_result {
            trace("auth", "rejected", "Authentication rejected by server", None, true);
            return Err(SshError::AuthenticationFailed(
                "Authentication rejected".to_string(),
            ));
        }

        trace("auth", "success", "Authentication successful", None, false);
        trace("ssh", "connected", &format!("SSH connection established to {}:{}", host, port), None, false);

        log::info!("SSH connection established to {}:{}", host, port);

        Ok(Self {
            handle,
            sftp: None,
            username: username.to_string(),
        })
    }

    /// Initialize SFTP subsystem
    async fn ensure_sftp(&mut self) -> Result<Arc<Mutex<SftpSession>>, SshError> {
        if let Some(sftp) = &self.sftp {
            return Ok(sftp.clone());
        }

        let channel = self
            .handle
            .channel_open_session()
            .await
            .map_err(|e| SshError::ChannelError(e.to_string()))?;

        channel
            .request_subsystem(true, "sftp")
            .await
            .map_err(|e| {
                SshError::SftpError(format!(
                    "Failed to start SFTP subsystem. Ensure the SSH server enables SFTP (OpenSSH: `Subsystem sftp ...`). Underlying error: {}",
                    e
                ))
            })?;

        // russh-sftp defaults to a 10s response timeout per request, which can be too aggressive
        // on mobile networks and/or large directories. Set a higher timeout before init.
        let sftp = SftpSession::new_opts(channel.into_stream(), Some(180))
            .await
            .map_err(|e| {
                SshError::SftpError(format!(
                    "Failed to initialize SFTP session. Underlying error: {}",
                    e
                ))
            })?;

        let sftp = Arc::new(Mutex::new(sftp));
        self.sftp = Some(sftp.clone());

        Ok(sftp)
    }

    /// Read file contents and return file stat in a single SFTP lock scope (reduces round trips from the UI).
    pub async fn read_file_with_stat(&mut self, path: &str) -> Result<(String, SftpStat), SshError> {
        match self.read_file_with_stat_once(path).await {
            Ok(result) => Ok(result),
            Err(SshError::SftpTimeout | SshError::SftpSessionClosed) => {
                self.reset_sftp();
                self.read_file_with_stat_once(path).await
            }
            Err(e) => Err(e),
        }
    }

    async fn read_file_with_stat_once(&mut self, path: &str) -> Result<(String, SftpStat), SshError> {
        let sftp = self.ensure_sftp().await?;
        let sftp = sftp.lock().await;

        let mut file = sftp.open(path).await.map_err(map_sftp_error)?;

        let mut content = Vec::new();
        file.read_to_end(&mut content)
            .await
            .map_err(|e| SshError::SftpError(e.to_string()))?;

        let metadata = sftp.metadata(path).await.map_err(map_sftp_error)?;

        let text = String::from_utf8(content).map_err(|e| SshError::SftpError(e.to_string()))?;
        let stat = SftpStat {
            size: metadata.size.unwrap_or(0),
            mtime: metadata.mtime.map(|t| t as i64).unwrap_or(0),
        };

        Ok((text, stat))
    }

    /// List directory contents
    pub async fn list_dir(&mut self, path: &str) -> Result<Vec<SftpEntry>, SshError> {
        match self.list_dir_once(path).await {
            Ok(entries) => Ok(entries),
            Err(SshError::SftpTimeout | SshError::SftpSessionClosed) => {
                // Recreate SFTP session and retry once; useful on flaky mobile networks.
                self.reset_sftp();
                self.list_dir_once(path).await
            }
            Err(e) => Err(e),
        }
    }

    async fn list_dir_once(&mut self, path: &str) -> Result<Vec<SftpEntry>, SshError> {
        let sftp = self.ensure_sftp().await?;
        let sftp = sftp.lock().await;

        let entries = sftp
            .read_dir(path)
            .await
            .map_err(map_sftp_error)?;

        let mut result = Vec::new();
        for entry in entries {
            let file_type = entry.file_type();
            let metadata = entry.metadata();

            result.push(SftpEntry {
                name: entry.file_name(),
                is_directory: file_type.is_dir(),
                size: metadata.size.unwrap_or(0),
                mtime: metadata.mtime.map(|t| t as i64).unwrap_or(0),
                permissions: metadata.permissions.map(|p| format!("{:o}", p)),
            });
        }

        Ok(result)
    }

    /// Read file contents
    pub async fn read_file(&mut self, path: &str) -> Result<String, SshError> {
        match self.read_file_once(path).await {
            Ok(content) => Ok(content),
            Err(SshError::SftpTimeout | SshError::SftpSessionClosed) => {
                self.reset_sftp();
                self.read_file_once(path).await
            }
            Err(e) => Err(e),
        }
    }

    async fn read_file_once(&mut self, path: &str) -> Result<String, SshError> {
        let sftp = self.ensure_sftp().await?;
        let sftp = sftp.lock().await;

        let mut file = sftp
            .open(path)
            .await
            .map_err(map_sftp_error)?;

        let mut content = Vec::new();
        file.read_to_end(&mut content)
            .await
            .map_err(|e| SshError::SftpError(e.to_string()))?;

        String::from_utf8(content).map_err(|e| SshError::SftpError(e.to_string()))
    }

    /// Write content to a file
    pub async fn write_file(&mut self, path: &str, content: &str) -> Result<(), SshError> {
        match self.write_file_once(path, content).await {
            Ok(()) => Ok(()),
            Err(SshError::SftpTimeout | SshError::SftpSessionClosed) => {
                self.reset_sftp();
                self.write_file_once(path, content).await
            }
            Err(e) => Err(e),
        }
    }

    async fn write_file_once(&mut self, path: &str, content: &str) -> Result<(), SshError> {
        let sftp = self.ensure_sftp().await?;
        let sftp = sftp.lock().await;

        let mut file = sftp
            .create(path)
            .await
            .map_err(map_sftp_error)?;

        file.write_all(content.as_bytes())
            .await
            .map_err(|e| SshError::SftpError(e.to_string()))?;

        Ok(())
    }

    /// Get file metadata
    pub async fn stat(&mut self, path: &str) -> Result<SftpStat, SshError> {
        match self.stat_once(path).await {
            Ok(stat) => Ok(stat),
            Err(SshError::SftpTimeout | SshError::SftpSessionClosed) => {
                self.reset_sftp();
                self.stat_once(path).await
            }
            Err(e) => Err(e),
        }
    }

    async fn stat_once(&mut self, path: &str) -> Result<SftpStat, SshError> {
        let sftp = self.ensure_sftp().await?;
        let sftp = sftp.lock().await;

        let metadata = sftp
            .metadata(path)
            .await
            .map_err(map_sftp_error)?;

        Ok(SftpStat {
            size: metadata.size.unwrap_or(0),
            mtime: metadata.mtime.map(|t| t as i64).unwrap_or(0),
        })
    }

    /// Get the home directory path
    pub async fn get_home_dir(&mut self) -> Result<String, SshError> {
        match self.get_home_dir_once().await {
            Ok(path) => Ok(path),
            Err(SshError::SftpTimeout | SshError::SftpSessionClosed) => {
                self.reset_sftp();
                self.get_home_dir_once().await
            }
            Err(e) => Err(e),
        }
    }

    async fn get_home_dir_once(&mut self) -> Result<String, SshError> {
        let sftp = self.ensure_sftp().await?;
        let sftp = sftp.lock().await;

        // Use SFTP canonicalize to resolve "." which gives us the current directory
        // (which is typically the home directory when first connected)
        let path = sftp
            .canonicalize(".")
            .await
            .map_err(map_sftp_error)?;

        Ok(path)
    }

    /// Create an empty file
    pub async fn create_file(&mut self, path: &str) -> Result<(), SshError> {
        match self.create_file_once(path).await {
            Ok(()) => Ok(()),
            Err(SshError::SftpTimeout | SshError::SftpSessionClosed) => {
                self.reset_sftp();
                self.create_file_once(path).await
            }
            Err(e) => Err(e),
        }
    }

    async fn create_file_once(&mut self, path: &str) -> Result<(), SshError> {
        let sftp = self.ensure_sftp().await?;
        let sftp = sftp.lock().await;

        let _file = sftp
            .create(path)
            .await
            .map_err(map_sftp_error)?;

        Ok(())
    }

    /// Create a directory
    pub async fn create_dir(&mut self, path: &str) -> Result<(), SshError> {
        match self.create_dir_once(path).await {
            Ok(()) => Ok(()),
            Err(SshError::SftpTimeout | SshError::SftpSessionClosed) => {
                self.reset_sftp();
                self.create_dir_once(path).await
            }
            Err(e) => Err(e),
        }
    }

    async fn create_dir_once(&mut self, path: &str) -> Result<(), SshError> {
        let sftp = self.ensure_sftp().await?;
        let sftp = sftp.lock().await;

        sftp.create_dir(path)
            .await
            .map_err(map_sftp_error)?;

        Ok(())
    }

    /// Delete a file or directory
    pub async fn delete(&mut self, path: &str) -> Result<(), SshError> {
        match self.delete_once(path).await {
            Ok(()) => Ok(()),
            Err(SshError::SftpTimeout | SshError::SftpSessionClosed) => {
                self.reset_sftp();
                self.delete_once(path).await
            }
            Err(e) => Err(e),
        }
    }

    async fn delete_once(&mut self, path: &str) -> Result<(), SshError> {
        let sftp = self.ensure_sftp().await?;
        let sftp = sftp.lock().await;

        // Try to remove as file first, then as directory
        if sftp.remove_file(path).await.is_err() {
            sftp.remove_dir(path)
                .await
                .map_err(map_sftp_error)?;
        }

        Ok(())
    }

    /// Rename/move a file or directory
    pub async fn rename(&mut self, old_path: &str, new_path: &str) -> Result<(), SshError> {
        match self.rename_once(old_path, new_path).await {
            Ok(()) => Ok(()),
            Err(SshError::SftpTimeout | SshError::SftpSessionClosed) => {
                self.reset_sftp();
                self.rename_once(old_path, new_path).await
            }
            Err(e) => Err(e),
        }
    }

    async fn rename_once(&mut self, old_path: &str, new_path: &str) -> Result<(), SshError> {
        let sftp = self.ensure_sftp().await?;
        let sftp = sftp.lock().await;

        sftp.rename(old_path, new_path)
            .await
            .map_err(map_sftp_error)?;

        Ok(())
    }

    /// Create a new PTY session
    pub async fn create_pty_session(
        &mut self,
        terminal_id: String,
        connection_id: String,
        app: AppHandle,
        working_dir: Option<String>,
    ) -> Result<PtySession, SshError> {
        let channel = self
            .handle
            .channel_open_session()
            .await
            .map_err(|e| SshError::ChannelError(e.to_string()))?;

        // Request PTY
        channel
            .request_pty(true, "xterm-256color", 80, 24, 640, 480, &[])
            .await
            .map_err(|e| SshError::ChannelError(e.to_string()))?;

        // Request shell
        channel
            .request_shell(true)
            .await
            .map_err(|e| SshError::ChannelError(e.to_string()))?;

        Ok(PtySession::new(terminal_id, connection_id, channel, app, working_dir))
    }

    /// Disconnect the SSH connection
    pub async fn disconnect(&mut self) -> Result<(), SshError> {
        self.reset_sftp();

        self.handle
            .disconnect(Disconnect::ByApplication, "User requested disconnect", "en")
            .await
            .map_err(|e| SshError::ConnectionFailed(e.to_string()))?;

        Ok(())
    }
}

fn map_sftp_error(error: SftpClientError) -> SshError {
    match error {
        SftpClientError::Timeout => SshError::SftpTimeout,
        SftpClientError::UnexpectedBehavior(msg) if msg.to_lowercase().contains("session closed") => {
            SshError::SftpSessionClosed
        }
        other => SshError::SftpError(other.to_string()),
    }
}
