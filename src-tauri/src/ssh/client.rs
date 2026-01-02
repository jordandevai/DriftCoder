use crate::ssh::auth::AuthMethod;
use crate::ssh::pty::PtySession;
use crate::ssh::sftp::{SftpEntry, SftpStat};
use async_trait::async_trait;
use russh::client::{self, Config, Handle, Handler};
use russh::Disconnect;
use russh_sftp::client::error::Error as SftpClientError;
use russh_sftp::client::SftpSession;
use ssh_key::public::PublicKey;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tauri::AppHandle;
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{lookup_host, TcpStream};
use tokio::sync::Mutex;

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
    HandshakeFailed { addr: SocketAddr, detail: String },
    #[error("SSH handshake to {addr} aborted (JoinError)")]
    HandshakeJoinAborted { addr: SocketAddr },
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
struct ClientHandler;

#[async_trait]
impl Handler for ClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &PublicKey,
    ) -> Result<bool, Self::Error> {
        // TODO: Implement proper host key verification
        // For now, accept all keys (not secure for production)
        Ok(true)
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
    pub async fn connect(
        host: &str,
        port: u16,
        username: &str,
        auth: AuthMethod,
    ) -> Result<Self, SshError> {
        let host = host.trim();
        let username = username.trim();

        let mut config = Config::default();
        // Mobile networks and some SFTP servers can be slow to respond; keepalives help
        // prevent idle connections from being dropped while the UI is loading.
        config.keepalive_interval = Some(Duration::from_secs(20));
        config.keepalive_max = 3;
        
        // Increase rekey time for mobile networks - rekeying can cause issues on unstable connections
        config.rekey_time = Some(Duration::from_secs(3600));
        config.rekey_limit = Some(1024 * 1024 * 1024); // 1GB
        
        let config = Arc::new(config);

        let mut resolved: Vec<std::net::SocketAddr> = lookup_host((host, port))
            .await
            .map_err(|e| SshError::DnsLookupFailed {
                host: host.to_string(),
                port,
                detail: e.to_string(),
            })?
            .collect();

        if resolved.is_empty() {
            return Err(SshError::ConnectionFailed(format!(
                "DNS lookup returned no addresses for {}:{}",
                host, port
            )));
        }

        // Prefer IPv4 to avoid IPv6-only / broken IPv6 routes on some networks.
        resolved.sort_by_key(|a| match a {
            std::net::SocketAddr::V4(_) => 0,
            std::net::SocketAddr::V6(_) => 1,
        });

        let mut last_error: Option<SshError> = None;
        let mut handle: Option<Handle<ClientHandler>> = None;

        for addr in resolved.iter().copied() {
            // Increased timeout for mobile/tablet networks with higher latency
            let socket = match tokio::time::timeout(Duration::from_secs(15), TcpStream::connect(addr))
                .await
            {
                Ok(Ok(s)) => s,
                Ok(Err(e)) => {
                    last_error = Some(SshError::TcpConnectFailed {
                        addr,
                        detail: e.to_string(),
                    });
                    continue;
                }
                Err(_) => {
                    last_error = Some(SshError::TcpConnectTimeout { addr });
                    continue;
                }
            };

            let _ = socket.set_nodelay(true);

            log::debug!("Starting SSH handshake to {}", addr);

            match client::connect_stream(config.clone(), socket, ClientHandler).await {
                Ok(h) => {
                    log::debug!("SSH handshake successful to {}", addr);
                    handle = Some(h);
                    break;
                }
                Err(e) => {
                    let msg = e.to_string();
                    log::warn!("SSH handshake failed to {}: {}", addr, msg);
                    if msg == "JoinError" {
                        last_error = Some(SshError::HandshakeJoinAborted { addr });
                    } else {
                        last_error = Some(SshError::HandshakeFailed {
                            addr,
                            detail: msg,
                        });
                    }
                    continue;
                }
            }
        }

        let mut handle = handle.ok_or_else(|| {
            last_error.unwrap_or_else(|| {
                SshError::ConnectionFailed("Failed to establish SSH connection".to_string())
            })
        })?;

        // Authenticate
        let auth_result = match &auth {
            AuthMethod::Password(password) => handle
                .authenticate_password(username, password)
                .await
                .map_err(|e| SshError::AuthenticationFailed(e.to_string()))?,
            AuthMethod::Key { .. } => {
                let key = auth
                    .load_key_pair()
                    .await
                    .map_err(|e| SshError::AuthenticationFailed(e.to_string()))?
                    .ok_or_else(|| {
                        SshError::AuthenticationFailed("No key pair loaded".to_string())
                    })?;

                handle
                    .authenticate_publickey(username, key)
                    .await
                    .map_err(|e| SshError::AuthenticationFailed(e.to_string()))?
            }
        };

        if !auth_result {
            return Err(SshError::AuthenticationFailed(
                "Authentication rejected".to_string(),
            ));
        }

        log::info!("SSH connection established to {}:{}", host, port);

        // Warmup period: Allow the connection to stabilize, especially important for
        // mobile/tablet networks where async runtime scheduling may be less predictable.
        // This prevents handshake aborts when the connection is immediately put under load.
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Verify the connection is still alive after warmup
        // This catches cases where the handshake completed but the connection dropped
        // during the warmup period (common on unstable mobile networks)
        if handle.is_closed() {
            return Err(SshError::ConnectionFailed(
                "Connection closed during warmup".to_string(),
            ));
        }

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
