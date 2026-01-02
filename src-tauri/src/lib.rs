mod commands;
mod credentials;
mod ipc_error;
mod ssh;
mod state;
pub mod trace;

use state::AppState;
use std::sync::Arc;
use tokio::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    let app_state = Arc::new(Mutex::new(AppState::new()));

    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            // Connection commands
            commands::connection::ssh_connect,
            commands::connection::ssh_disconnect,
            commands::connection::ssh_get_home_dir,
            commands::connection::ssh_test_connection,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
