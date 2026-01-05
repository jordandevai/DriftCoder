use serde::de::DeserializeOwned;
use tauri::{
    plugin::{PluginApi, PluginHandle},
    AppHandle, Runtime,
};

use crate::Result;

#[cfg(target_os = "android")]
const PLUGIN_IDENTIFIER: &str = "app.tauri.connectionpersistence";

#[cfg(target_os = "ios")]
tauri::ios_plugin_binding!(init_plugin_connection_persistence);

pub struct ConnectionPersistence<R: Runtime>(Option<PluginHandle<R>>);

pub fn init<R: Runtime, C: DeserializeOwned>(
    _app: &AppHandle<R>,
    api: PluginApi<R, C>,
) -> Result<ConnectionPersistence<R>> {
    #[cfg(target_os = "android")]
    let handle = match api.register_android_plugin(PLUGIN_IDENTIFIER, "ConnectionPersistencePlugin") {
        Ok(handle) => Some(handle),
        Err(e) => {
            // Don't crash the whole app if the mobile plugin fails to load (e.g. build misconfiguration).
            log::warn!("[connection_persistence] Android plugin registration failed: {e}");
            None
        }
    };
    #[cfg(target_os = "ios")]
    let handle = match api.register_ios_plugin(init_plugin_connection_persistence) {
        Ok(handle) => Some(handle),
        Err(e) => {
            log::warn!("[connection_persistence] iOS plugin registration failed: {e}");
            None
        }
    };
    Ok(ConnectionPersistence(handle))
}

impl<R: Runtime> ConnectionPersistence<R> {
    pub async fn start_background_mode(&self) -> Result<()> {
        let Some(handle) = &self.0 else { return Ok(()); };
        // Resolve with an empty object.
        let _: serde_json::Value = handle
            .run_mobile_plugin_async("start", serde_json::json!({}))
            .await?;
        Ok(())
    }

    pub async fn stop_background_mode(&self) -> Result<()> {
        let Some(handle) = &self.0 else { return Ok(()); };
        let _: serde_json::Value = handle
            .run_mobile_plugin_async("stop", serde_json::json!({}))
            .await?;
        Ok(())
    }

    #[cfg(target_os = "android")]
    pub async fn consume_disconnect_request(&self) -> Result<bool> {
        let Some(handle) = &self.0 else { return Ok(false); };
        #[derive(serde::Deserialize)]
        struct Resp {
            requested: bool,
        }
        let resp: Resp = handle
            .run_mobile_plugin_async("consumeDisconnectRequest", serde_json::json!({}))
            .await?;
        Ok(resp.requested)
    }

    #[cfg(target_os = "android")]
    pub async fn set_active(&self, active: bool) -> Result<()> {
        let Some(handle) = &self.0 else { return Ok(()); };
        let _: serde_json::Value = handle
            .run_mobile_plugin_async("setActive", serde_json::json!({ "active": active }))
            .await?;
        Ok(())
    }
}
