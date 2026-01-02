mod commands;
mod credentials;
mod diagnostics;
mod ipc_error;
mod ssh;
mod state;
pub mod trace;

use state::AppState;
use std::sync::Arc;
use tauri::{image::Image, Manager, RunEvent};
use tokio::sync::Mutex;
use trace::{emit_trace, is_trace_enabled, TraceEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    diagnostics::install_panic_hook();
    env_logger::init();

    let app_state = Arc::new(Mutex::new(AppState::new()));

    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .manage(app_state)
        .setup(|app| {
            // Set window icon for Linux dev mode (production builds use bundle icons)
            #[cfg(target_os = "linux")]
            {
                if let Some(window) = app.get_webview_window("main") {
                    let icon_bytes = include_bytes!("../icons/icon.png");
                    if let Ok(icon) = Image::from_bytes(icon_bytes) {
                        let _ = window.set_icon(icon);
                    }
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Connection commands
            commands::connection::ssh_connect,
            commands::connection::ssh_disconnect,
            commands::connection::ssh_get_home_dir,
            commands::connection::ssh_test_connection,
            commands::connection::ssh_list_trusted_host_keys,
            commands::connection::ssh_trust_host_key,
            commands::connection::ssh_forget_host_key,
            // File system commands
            commands::filesystem::sftp_list_dir,
            commands::filesystem::sftp_read_file,
            commands::filesystem::sftp_read_file_with_stat,
            commands::filesystem::sftp_write_file,
            commands::filesystem::sftp_stat,
            commands::filesystem::sftp_create_file,
            commands::filesystem::sftp_create_dir,
            commands::filesystem::sftp_delete,
            commands::filesystem::sftp_rename,
            // Terminal commands
            commands::terminal::terminal_create,
            commands::terminal::terminal_write,
            commands::terminal::terminal_resize,
            commands::terminal::terminal_close,
            // Debug commands
            commands::debug::debug_enable_trace,
            commands::debug::debug_disable_trace,
            commands::debug::debug_is_trace_enabled,
            commands::debug::debug_export_diagnostics,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            // Capture app lifecycle events for debugging
            if is_trace_enabled() {
                match &event {
                    RunEvent::Ready => {
                        emit_trace(app, TraceEvent::new("app", "ready", "Application ready"));
                    }
                    RunEvent::Resumed => {
                        // Android/iOS: App returned to foreground
                        emit_trace(app, TraceEvent::new("app", "resumed", "App resumed from background (mobile)"));
                        log::info!("[LIFECYCLE] App resumed");
                    }
                    RunEvent::ExitRequested { api, .. } => {
                        emit_trace(app, TraceEvent::new("app", "exit_requested", "Exit requested"));
                        log::info!("[LIFECYCLE] Exit requested");
                        let _ = api;
                    }
                    RunEvent::Exit => {
                        emit_trace(app, TraceEvent::new("app", "exit", "Application exiting"));
                        log::info!("[LIFECYCLE] App exiting");
                    }
                    _ => {
                        // Log any other events for debugging (includes platform-specific events)
                        log::debug!("[LIFECYCLE] Other event: {:?}", event);
                    }
                }
            }
        });
}
