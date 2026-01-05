use crate::ipc_error::IpcError;
use tauri::AppHandle;

#[cfg(target_os = "android")]
use tauri_plugin_connection_persistence::ConnectionPersistenceExt;

#[tauri::command]
pub async fn android_persistence_start(app: AppHandle) -> Result<(), IpcError> {
    #[cfg(target_os = "android")]
    {
        app.connection_persistence()
            .start_background_mode()
            .await
            .map_err(|e| IpcError::new("android_persistence_start_failed", "Failed to start background persistence").with_raw(e.to_string()))?;
    }
    #[cfg(not(target_os = "android"))]
    let _ = app;
    Ok(())
}

#[tauri::command]
pub async fn android_persistence_stop(app: AppHandle) -> Result<(), IpcError> {
    #[cfg(target_os = "android")]
    {
        app.connection_persistence()
            .stop_background_mode()
            .await
            .map_err(|e| IpcError::new("android_persistence_stop_failed", "Failed to stop background persistence").with_raw(e.to_string()))?;
    }
    #[cfg(not(target_os = "android"))]
    let _ = app;
    Ok(())
}

#[tauri::command]
pub async fn android_persistence_consume_disconnect_request(app: AppHandle) -> Result<bool, IpcError> {
    #[cfg(target_os = "android")]
    {
        return app
            .connection_persistence()
            .consume_disconnect_request()
            .await
            .map_err(|e| {
                IpcError::new(
                    "android_persistence_consume_failed",
                    "Failed to read disconnect request",
                )
                .with_raw(e.to_string())
            });
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = app;
        Ok(false)
    }
}
