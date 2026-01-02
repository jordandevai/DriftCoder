//! Debug commands for development and troubleshooting.

use crate::trace;

/// Enable connection tracing at runtime
#[tauri::command]
pub fn debug_enable_trace() -> bool {
    trace::enable_trace();
    true
}

/// Disable connection tracing at runtime
#[tauri::command]
pub fn debug_disable_trace() -> bool {
    trace::disable_trace();
    false
}

/// Check if connection tracing is enabled
#[tauri::command]
pub fn debug_is_trace_enabled() -> bool {
    trace::is_trace_enabled()
}
