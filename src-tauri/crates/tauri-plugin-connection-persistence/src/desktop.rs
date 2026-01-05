use tauri::{AppHandle, Runtime};

pub struct ConnectionPersistence<R: Runtime> {
    #[allow(dead_code)]
    app: AppHandle<R>,
}

pub fn init<R: Runtime>(app: &AppHandle<R>) -> crate::Result<ConnectionPersistence<R>> {
    Ok(ConnectionPersistence { app: app.clone() })
}

