use serde::{Deserialize, Serialize};

/// Represents a file/directory entry from SFTP
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SftpEntry {
    pub name: String,
    pub is_directory: bool,
    pub size: u64,
    pub mtime: i64,
    pub permissions: Option<String>,
}

/// File metadata from SFTP
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SftpStat {
    pub size: u64,
    pub mtime: i64,
}
