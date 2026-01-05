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

pub struct ConnectionPersistence<R: Runtime>(PluginHandle<R>);

pub fn init<R: Runtime, C: DeserializeOwned>(
    _app: &AppHandle<R>,
    api: PluginApi<R, C>,
) -> Result<ConnectionPersistence<R>> {
    #[cfg(target_os = "android")]
    let handle = api.register_android_plugin(PLUGIN_IDENTIFIER, "ConnectionPersistencePlugin")?;
    #[cfg(target_os = "ios")]
    let handle = api.register_ios_plugin(init_plugin_connection_persistence)?;
    Ok(ConnectionPersistence(handle))
}

impl<R: Runtime> ConnectionPersistence<R> {
    pub async fn start_background_mode(&self) -> Result<()> {
        // Resolve with an empty object.
        let _: serde_json::Value = self
            .0
            .run_mobile_plugin_async("start", serde_json::json!({}))
            .await?;
        Ok(())
    }

    pub async fn stop_background_mode(&self) -> Result<()> {
        let _: serde_json::Value = self
            .0
            .run_mobile_plugin_async("stop", serde_json::json!({}))
            .await?;
        Ok(())
    }

    #[cfg(target_os = "android")]
    pub async fn consume_disconnect_request(&self) -> Result<bool> {
        #[derive(serde::Deserialize)]
        struct Resp {
            requested: bool,
        }
        let resp: Resp = self
            .0
            .run_mobile_plugin_async("consumeDisconnectRequest", serde_json::json!({}))
            .await?;
        Ok(resp.requested)
    }
}
