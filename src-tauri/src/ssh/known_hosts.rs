use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tauri::AppHandle;
use tauri::Manager;
use tokio::fs;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnownHostEntry {
    pub host: String,
    pub port: u16,
    pub key_type: String,
    pub fingerprint_sha256: String,
    pub public_key_openssh: String,
    pub trusted_at: u64,
}

#[derive(Default)]
struct KnownHostsState {
    loaded: bool,
    entries: HashMap<String, KnownHostEntry>,
}

static KNOWN_HOSTS: std::sync::OnceLock<Mutex<KnownHostsState>> = std::sync::OnceLock::new();

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn key(host: &str, port: u16) -> String {
    format!("{}:{}", host.trim(), port)
}

fn file_path(app: &AppHandle) -> Result<PathBuf, tauri::Error> {
    Ok(app.path().app_config_dir()?.join("known_hosts.json"))
}

async fn ensure_loaded_locked(app: &AppHandle, state: &mut KnownHostsState) -> Result<(), String> {
    if state.loaded {
        return Ok(());
    }

    let path = file_path(app).map_err(|e| e.to_string())?;
    let bytes = match fs::read(&path).await {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            state.loaded = true;
            return Ok(());
        }
        Err(e) => return Err(e.to_string()),
    };

    let decoded: HashMap<String, KnownHostEntry> =
        serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    state.entries = decoded;
    state.loaded = true;
    Ok(())
}

async fn save_locked(app: &AppHandle, state: &KnownHostsState) -> Result<(), String> {
    let path = file_path(app).map_err(|e| e.to_string())?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await.map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_vec_pretty(&state.entries).map_err(|e| e.to_string())?;
    fs::write(path, json).await.map_err(|e| e.to_string())
}

fn store() -> &'static Mutex<KnownHostsState> {
    KNOWN_HOSTS.get_or_init(|| Mutex::new(KnownHostsState::default()))
}

pub async fn get(app: &AppHandle, host: &str, port: u16) -> Result<Option<KnownHostEntry>, String> {
    let mut guard = store().lock().await;
    ensure_loaded_locked(app, &mut guard).await?;
    Ok(guard.entries.get(&key(host, port)).cloned())
}

pub async fn upsert(
    app: &AppHandle,
    host: &str,
    port: u16,
    key_type: &str,
    fingerprint_sha256: &str,
    public_key_openssh: &str,
) -> Result<(), String> {
    let mut guard = store().lock().await;
    ensure_loaded_locked(app, &mut guard).await?;
    guard.entries.insert(
        key(host, port),
        KnownHostEntry {
            host: host.trim().to_string(),
            port,
            key_type: key_type.to_string(),
            fingerprint_sha256: fingerprint_sha256.to_string(),
            public_key_openssh: public_key_openssh.to_string(),
            trusted_at: now_ms(),
        },
    );
    save_locked(app, &guard).await
}

pub async fn remove(app: &AppHandle, host: &str, port: u16) -> Result<(), String> {
    let mut guard = store().lock().await;
    ensure_loaded_locked(app, &mut guard).await?;
    guard.entries.remove(&key(host, port));
    save_locked(app, &guard).await
}

pub async fn list(app: &AppHandle) -> Result<Vec<KnownHostEntry>, String> {
    let mut guard = store().lock().await;
    ensure_loaded_locked(app, &mut guard).await?;
    Ok(guard.entries.values().cloned().collect())
}
