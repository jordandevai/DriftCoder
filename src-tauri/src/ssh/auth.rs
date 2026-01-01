#![allow(dead_code)]
use ssh_key::PrivateKey;
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Failed to read key file: {0}")]
    KeyFileRead(String),
    #[error("Failed to parse key: {0}")]
    KeyParse(String),
    #[error("Key requires passphrase")]
    PassphraseRequired,
    #[error("Invalid passphrase")]
    InvalidPassphrase,
}

/// Authentication method for SSH connections
#[derive(Debug, Clone)]
pub enum AuthMethod {
    /// Password authentication
    Password(String),
    /// Key-based authentication
    Key {
        path: String,
        passphrase: Option<String>,
    },
}

impl AuthMethod {
    /// Load the key pair for key-based authentication
    pub async fn load_key_pair(&self) -> Result<Option<Arc<PrivateKey>>, AuthError> {
        match self {
            AuthMethod::Password(_) => Ok(None),
            AuthMethod::Key { path, passphrase } => {
                let key_path = Path::new(path);

                // Expand ~ to home directory
                let expanded_path = if path.starts_with("~/") {
                    if let Some(home) = dirs::home_dir() {
                        home.join(&path[2..])
                    } else {
                        key_path.to_path_buf()
                    }
                } else {
                    key_path.to_path_buf()
                };

                let key_data = tokio::fs::read_to_string(&expanded_path)
                    .await
                    .map_err(|e| AuthError::KeyFileRead(e.to_string()))?;

                let key = if let Some(pass) = passphrase {
                    PrivateKey::from_openssh(&key_data)
                        .map_err(|_| AuthError::KeyParse("Failed to parse key".to_string()))?
                        .decrypt(pass.as_bytes())
                        .map_err(|_| AuthError::InvalidPassphrase)?
                } else {
                    PrivateKey::from_openssh(&key_data).map_err(|e| {
                        let msg = e.to_string();
                        if msg.contains("encrypted") || msg.contains("passphrase") {
                            AuthError::PassphraseRequired
                        } else {
                            AuthError::KeyParse(msg)
                        }
                    })?
                };

                Ok(Some(Arc::new(key)))
            }
        }
    }

    /// Get the password for password authentication
    #[allow(dead_code)]
    pub fn password(&self) -> Option<&str> {
        match self {
            AuthMethod::Password(pass) => Some(pass),
            AuthMethod::Key { .. } => None,
        }
    }
}
