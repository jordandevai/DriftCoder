use crate::ssh::auth::AuthMethod;
use crate::ssh::pty::PtySession;
use crate::ssh::sftp::{SftpEntry, SftpStat};
use async_trait::async_trait;
use russh::client::{self, Config, Handle, Handler};
use russh::Disconnect;
use russh_sftp::client::SftpSession;
use ssh_key::public::PublicKey;
use std::sync::Arc;
use tauri::AppHandle;
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;

#[derive(Debug, Error)]
pub enum SshError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),
    #[error("SFTP error: {0}")]
    SftpError(String),
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
    /// Establish a new SSH connection
    pub async fn connect(
        host: &str,
        port: u16,
        username: &str,
        auth: AuthMethod,
    ) -> Result<Self, SshError> {
        let config = Config::default();
        let config = Arc::new(config);
        let handler = ClientHandler;

        let addr = format!("{}:{}", host, port);

        let mut handle = client::connect(config, &addr, handler)
            .await
            .map_err(|e| SshError::ConnectionFailed(e.to_string()))?;

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
            .map_err(|e| SshError::SftpError(e.to_string()))?;

        let sftp = SftpSession::new(channel.into_stream())
            .await
            .map_err(|e| SshError::SftpError(e.to_string()))?;

        let sftp = Arc::new(Mutex::new(sftp));
        self.sftp = Some(sftp.clone());

        Ok(sftp)
    }

    /// List directory contents
    pub async fn list_dir(&mut self, path: &str) -> Result<Vec<SftpEntry>, SshError> {
        let sftp = self.ensure_sftp().await?;
        let sftp = sftp.lock().await;

        let entries = sftp
            .read_dir(path)
            .await
            .map_err(|e| SshError::SftpError(e.to_string()))?;

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
        let sftp = self.ensure_sftp().await?;
        let sftp = sftp.lock().await;

        let mut file = sftp
            .open(path)
            .await
            .map_err(|e| SshError::SftpError(e.to_string()))?;

        let mut content = Vec::new();
        file.read_to_end(&mut content)
            .await
            .map_err(|e| SshError::SftpError(e.to_string()))?;

        String::from_utf8(content).map_err(|e| SshError::SftpError(e.to_string()))
    }

    /// Write content to a file
    pub async fn write_file(&mut self, path: &str, content: &str) -> Result<(), SshError> {
        let sftp = self.ensure_sftp().await?;
        let sftp = sftp.lock().await;

        let mut file = sftp
            .create(path)
            .await
            .map_err(|e| SshError::SftpError(e.to_string()))?;

        file.write_all(content.as_bytes())
            .await
            .map_err(|e| SshError::SftpError(e.to_string()))?;

        Ok(())
    }

    /// Get file metadata
    pub async fn stat(&mut self, path: &str) -> Result<SftpStat, SshError> {
        let sftp = self.ensure_sftp().await?;
        let sftp = sftp.lock().await;

        let metadata = sftp
            .metadata(path)
            .await
            .map_err(|e| SshError::SftpError(e.to_string()))?;

        Ok(SftpStat {
            size: metadata.size.unwrap_or(0),
            mtime: metadata.mtime.map(|t| t as i64).unwrap_or(0),
        })
    }

    /// Get the home directory path
    pub async fn get_home_dir(&mut self) -> Result<String, SshError> {
        let sftp = self.ensure_sftp().await?;
        let sftp = sftp.lock().await;

        // Use SFTP canonicalize to resolve "." which gives us the current directory
        // (which is typically the home directory when first connected)
        let path = sftp
            .canonicalize(".")
            .await
            .map_err(|e| SshError::SftpError(e.to_string()))?;

        Ok(path)
    }

    /// Create an empty file
    pub async fn create_file(&mut self, path: &str) -> Result<(), SshError> {
        let sftp = self.ensure_sftp().await?;
        let sftp = sftp.lock().await;

        let _file = sftp
            .create(path)
            .await
            .map_err(|e| SshError::SftpError(e.to_string()))?;

        Ok(())
    }

    /// Create a directory
    pub async fn create_dir(&mut self, path: &str) -> Result<(), SshError> {
        let sftp = self.ensure_sftp().await?;
        let sftp = sftp.lock().await;

        sftp.create_dir(path)
            .await
            .map_err(|e| SshError::SftpError(e.to_string()))?;

        Ok(())
    }

    /// Delete a file or directory
    pub async fn delete(&mut self, path: &str) -> Result<(), SshError> {
        let sftp = self.ensure_sftp().await?;
        let sftp = sftp.lock().await;

        // Try to remove as file first, then as directory
        if sftp.remove_file(path).await.is_err() {
            sftp.remove_dir(path)
                .await
                .map_err(|e| SshError::SftpError(e.to_string()))?;
        }

        Ok(())
    }

    /// Rename/move a file or directory
    pub async fn rename(&mut self, old_path: &str, new_path: &str) -> Result<(), SshError> {
        let sftp = self.ensure_sftp().await?;
        let sftp = sftp.lock().await;

        sftp.rename(old_path, new_path)
            .await
            .map_err(|e| SshError::SftpError(e.to_string()))?;

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
        self.sftp = None;

        self.handle
            .disconnect(Disconnect::ByApplication, "User requested disconnect", "en")
            .await
            .map_err(|e| SshError::ConnectionFailed(e.to_string()))?;

        Ok(())
    }
}
