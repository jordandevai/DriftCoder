//! Connection tracing module for real-time debugging.
//!
//! Enable tracing by setting the environment variable `DRIFTCODE_DEBUG_TRACE=1`.
//! Trace events are emitted to the frontend via Tauri events and displayed in notifications.

use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use tauri::{AppHandle, Emitter};

/// Global flag to enable/disable tracing (checked once at startup)
static TRACE_ENABLED: OnceLock<bool> = OnceLock::new();

/// Atomic flag that can be toggled at runtime via command
static TRACE_RUNTIME_ENABLED: AtomicBool = AtomicBool::new(false);

/// Check if tracing is enabled (env var or runtime toggle)
pub fn is_trace_enabled() -> bool {
    let env_enabled = *TRACE_ENABLED.get_or_init(|| {
        std::env::var("DRIFTCODE_DEBUG_TRACE")
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(false)
    });
    env_enabled || TRACE_RUNTIME_ENABLED.load(Ordering::Relaxed)
}

/// Enable tracing at runtime
pub fn enable_trace() {
    TRACE_RUNTIME_ENABLED.store(true, Ordering::Relaxed);
    log::info!("Connection tracing enabled");
}

/// Disable tracing at runtime
pub fn disable_trace() {
    TRACE_RUNTIME_ENABLED.store(false, Ordering::Relaxed);
    log::info!("Connection tracing disabled");
}

/// Trace event payload sent to frontend
#[derive(Clone, Serialize)]
pub struct TraceEvent {
    /// Timestamp in milliseconds since UNIX epoch
    pub timestamp: u64,
    /// Category of the trace (e.g., "dns", "tcp", "ssh", "sftp")
    pub category: String,
    /// Short label for the step (e.g., "lookup", "connect", "handshake")
    pub step: String,
    /// Human-readable message
    pub message: String,
    /// Additional context (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// Whether this is an error trace
    pub is_error: bool,
}

impl TraceEvent {
    pub fn new(category: &str, step: &str, message: &str) -> Self {
        Self {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            category: category.to_string(),
            step: step.to_string(),
            message: message.to_string(),
            detail: None,
            is_error: false,
        }
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn error(mut self) -> Self {
        self.is_error = true;
        self
    }
}

/// Emit a trace event to the frontend (no-op if tracing disabled)
pub fn emit_trace(app: &AppHandle, event: TraceEvent) {
    if !is_trace_enabled() {
        return;
    }

    // Also log to stdout for backend debugging
    if event.is_error {
        log::warn!(
            "[TRACE] {}:{} - {} {}",
            event.category,
            event.step,
            event.message,
            event.detail.as_deref().unwrap_or("")
        );
    } else {
        log::info!(
            "[TRACE] {}:{} - {} {}",
            event.category,
            event.step,
            event.message,
            event.detail.as_deref().unwrap_or("")
        );
    }

    if let Err(e) = app.emit("connection_trace", event) {
        log::error!("Failed to emit trace event: {}", e);
    }
}

/// Convenience macro for emitting trace events
#[macro_export]
macro_rules! trace_emit {
    ($app:expr, $category:expr, $step:expr, $msg:expr) => {
        $crate::trace::emit_trace(
            $app,
            $crate::trace::TraceEvent::new($category, $step, $msg),
        )
    };
    ($app:expr, $category:expr, $step:expr, $msg:expr, $detail:expr) => {
        $crate::trace::emit_trace(
            $app,
            $crate::trace::TraceEvent::new($category, $step, $msg).with_detail($detail),
        )
    };
}

/// Convenience macro for emitting error trace events
#[macro_export]
macro_rules! trace_error {
    ($app:expr, $category:expr, $step:expr, $msg:expr) => {
        $crate::trace::emit_trace(
            $app,
            $crate::trace::TraceEvent::new($category, $step, $msg).error(),
        )
    };
    ($app:expr, $category:expr, $step:expr, $msg:expr, $detail:expr) => {
        $crate::trace::emit_trace(
            $app,
            $crate::trace::TraceEvent::new($category, $step, $msg)
                .with_detail($detail)
                .error(),
        )
    };
}
