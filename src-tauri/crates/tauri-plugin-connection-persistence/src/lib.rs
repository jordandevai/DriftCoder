use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};

mod error;

#[cfg(any(target_os = "android", target_os = "ios"))]
mod mobile;

#[cfg(not(any(target_os = "android", target_os = "ios")))]
mod desktop;

pub use error::{Error, Result};

/// Runtime state for Android/iOS background persistence hooks.
///
/// On Android this controls a foreground service used to keep the process alive while the app is backgrounded.
pub struct ConnectionPersistence<R: Runtime>(inner::Inner<R>);

mod inner {
    use super::*;

    #[cfg(any(target_os = "android", target_os = "ios"))]
    pub type Inner<R> = mobile::ConnectionPersistence<R>;

    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    pub type Inner<R> = desktop::ConnectionPersistence<R>;
}

pub trait ConnectionPersistenceExt<R: Runtime> {
    fn connection_persistence(&self) -> &ConnectionPersistence<R>;
}

impl<R: Runtime, T: Manager<R>> ConnectionPersistenceExt<R> for T {
    fn connection_persistence(&self) -> &ConnectionPersistence<R> {
        self.state::<ConnectionPersistence<R>>().inner()
    }
}

impl<R: Runtime> ConnectionPersistence<R> {
    #[cfg(any(target_os = "android", target_os = "ios"))]
    pub async fn start_background_mode(&self) -> Result<()> {
        self.0.start_background_mode().await
    }

    #[cfg(any(target_os = "android", target_os = "ios"))]
    pub async fn stop_background_mode(&self) -> Result<()> {
        self.0.stop_background_mode().await
    }

    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    pub async fn start_background_mode(&self) -> Result<()> {
        Ok(())
    }

    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    pub async fn stop_background_mode(&self) -> Result<()> {
        Ok(())
    }

    #[cfg(target_os = "android")]
    pub async fn consume_disconnect_request(&self) -> Result<bool> {
        self.0.consume_disconnect_request().await
    }

    #[cfg(not(target_os = "android"))]
    pub async fn consume_disconnect_request(&self) -> Result<bool> {
        Ok(false)
    }
}

/// Initializes the plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("connection_persistence")
        .setup(|app, api| {
            #[cfg(any(target_os = "android", target_os = "ios"))]
            let persistence = mobile::init(app, api)?;
            #[cfg(not(any(target_os = "android", target_os = "ios")))]
            let _ = api;
            #[cfg(not(any(target_os = "android", target_os = "ios")))]
            let persistence = desktop::init(app)?;
            app.manage(ConnectionPersistence(persistence));
            Ok(())
        })
        .build()
}
