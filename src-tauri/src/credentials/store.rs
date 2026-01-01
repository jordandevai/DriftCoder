#![allow(dead_code)]
use keyring::Entry;
use thiserror::Error;

#[allow(dead_code)]
const SERVICE_NAME: &str = "driftcode";

#[derive(Debug, Error)]
#[allow(dead_code)]
pub enum CredentialError {
    #[error("Failed to store credential: {0}")]
    StoreFailed(String),
    #[error("Failed to retrieve credential: {0}")]
    RetrieveFailed(String),
    #[error("Credential not found")]
    NotFound,
}

/// Credential store for securely storing passwords and passphrases
#[allow(dead_code)]
pub struct CredentialStore;

#[allow(dead_code)]
impl CredentialStore {
    /// Store a password for a connection
    pub fn store_password(connection_id: &str, password: &str) -> Result<(), CredentialError> {
        let entry = Entry::new(SERVICE_NAME, connection_id)
            .map_err(|e| CredentialError::StoreFailed(e.to_string()))?;

        entry
            .set_password(password)
            .map_err(|e| CredentialError::StoreFailed(e.to_string()))?;

        Ok(())
    }

    /// Retrieve a stored password
    pub fn get_password(connection_id: &str) -> Result<String, CredentialError> {
        let entry = Entry::new(SERVICE_NAME, connection_id)
            .map_err(|e| CredentialError::RetrieveFailed(e.to_string()))?;

        entry.get_password().map_err(|e| {
            if e.to_string().contains("No matching entry") {
                CredentialError::NotFound
            } else {
                CredentialError::RetrieveFailed(e.to_string())
            }
        })
    }

    /// Delete a stored password
    pub fn delete_password(connection_id: &str) -> Result<(), CredentialError> {
        let entry = Entry::new(SERVICE_NAME, connection_id)
            .map_err(|e| CredentialError::StoreFailed(e.to_string()))?;

        entry
            .delete_credential()
            .map_err(|e| CredentialError::StoreFailed(e.to_string()))?;

        Ok(())
    }

    /// Store a key passphrase
    pub fn store_key_passphrase(key_path: &str, passphrase: &str) -> Result<(), CredentialError> {
        let key = format!("key:{}", key_path);
        Self::store_password(&key, passphrase)
    }

    /// Retrieve a stored key passphrase
    pub fn get_key_passphrase(key_path: &str) -> Result<String, CredentialError> {
        let key = format!("key:{}", key_path);
        Self::get_password(&key)
    }
}
