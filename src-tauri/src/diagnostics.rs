use crate::trace::TraceEvent;
use serde::Serialize;
use serde_json::{json, Value};
use std::backtrace::Backtrace;
use std::collections::VecDeque;
use std::sync::{Mutex, OnceLock};

const TRACE_BUFFER_MAX: usize = 400;
const CONNECT_ATTEMPT_BUFFER_MAX: usize = 50;
const PANIC_BUFFER_MAX: usize = 10;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PanicRecord {
    pub timestamp: u64,
    pub message: String,
    pub location: Option<String>,
    pub backtrace: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectAttemptRecord {
    pub timestamp: u64,
    pub attempt_id: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub addr: Option<String>,
    pub resolved_addrs: Vec<String>,
    pub client_id: Option<String>,
    pub server_id: Option<String>,
    pub bytes_written: u64,
    pub bytes_read: u64,
    pub outcome: String,
    pub outcome_detail: Option<String>,
}

#[derive(Default)]
struct DiagnosticsState {
    traces: VecDeque<TraceEvent>,
    connect_attempts: VecDeque<ConnectAttemptRecord>,
    panics: VecDeque<PanicRecord>,
}

static DIAGNOSTICS: OnceLock<Mutex<DiagnosticsState>> = OnceLock::new();
static PANIC_HOOK_INSTALLED: OnceLock<()> = OnceLock::new();

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn state() -> &'static Mutex<DiagnosticsState> {
    DIAGNOSTICS.get_or_init(|| Mutex::new(DiagnosticsState::default()))
}

fn push_bounded<T>(deque: &mut VecDeque<T>, max: usize, value: T) {
    if deque.len() >= max {
        deque.pop_front();
    }
    deque.push_back(value);
}

pub fn record_trace(event: &TraceEvent) {
    let mut guard = state().lock().unwrap_or_else(|e| e.into_inner());
    push_bounded(&mut guard.traces, TRACE_BUFFER_MAX, event.clone());
}

pub fn record_connect_attempt(record: ConnectAttemptRecord) {
    let mut guard = state().lock().unwrap_or_else(|e| e.into_inner());
    push_bounded(
        &mut guard.connect_attempts,
        CONNECT_ATTEMPT_BUFFER_MAX,
        record,
    );
}

pub fn record_panic(record: PanicRecord) {
    let mut guard = state().lock().unwrap_or_else(|e| e.into_inner());
    push_bounded(&mut guard.panics, PANIC_BUFFER_MAX, record);
}

pub fn export() -> Value {
    let guard = state().lock().unwrap_or_else(|e| e.into_inner());
    json!({
        "generatedAt": now_ms(),
        "app": {
            "name": env!("CARGO_PKG_NAME"),
            "version": env!("CARGO_PKG_VERSION"),
        },
        "platform": {
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
        },
        "panics": guard.panics.iter().cloned().collect::<Vec<_>>(),
        "connectAttempts": guard.connect_attempts.iter().cloned().collect::<Vec<_>>(),
        "traces": guard.traces.iter().cloned().collect::<Vec<_>>(),
    })
}

pub fn install_panic_hook() {
    if PANIC_HOOK_INSTALLED.set(()).is_err() {
        return;
    }

    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let message = if let Some(s) = info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "panic payload (non-string)".to_string()
        };

        let location = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()));

        let backtrace = Some(format!("{:?}", Backtrace::force_capture()));

        record_panic(PanicRecord {
            timestamp: now_ms(),
            message: message.clone(),
            location,
            backtrace,
        });

        log::error!("[PANIC] {}", message);

        previous(info);
    }));
}

