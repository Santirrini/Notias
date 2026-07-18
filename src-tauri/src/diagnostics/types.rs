//! Wire types for the diagnostics module. Kept here (not in `commands.rs`)
//! so both the backend and the auto-generated TS bindings can share the
//! shape without a circular dependency on the command handlers.

use serde::Serialize;

/// Severity of a captured log entry. The strings match `tracing::Level`.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            LogLevel::Trace => "trace",
            LogLevel::Debug => "debug",
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error",
        }
    }
}

/// A single log line, parsed from the on-disk `notias.YYYY-MM-DD.log`.
/// Kept intentionally minimal: timestamp + level + module + message.
/// Full backtrace (when present) is folded into `message` so the wire
/// payload never loses context.
#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    /// Unix epoch milliseconds — easy to sort, format in the UI.
    pub ts_ms: i64,
    pub level: LogLevel,
    /// Originating module/target (e.g. `"notias_lib::notes::sync"`).
    pub target: String,
    pub message: String,
}

/// Snapshot of runtime metadata the diagnostics panel shows in its header.
/// Cheap to compute (no DB scan) so the UI can poll it freely.
#[derive(Debug, Clone, Serialize)]
pub struct DiagnosticsSnapshot {
    pub app_version: String,
    pub os: String,
    pub arch: String,
    /// Absolute path to the OS-specific data dir (`%APPDATA%\Notias\Notias`
    /// on Windows). Useful for the "Reveal in Explorer" copy-to-clipboard.
    pub data_dir: String,
    pub logs_dir: String,
    pub reports_dir: String,
    /// Total bytes used by `*.log` files (rolling, so capped by age).
    pub logs_bytes: u64,
    /// Total bytes of `crash.log` (0 if absent).
    pub crash_bytes: u64,
    /// Number of panic lines captured since the app started (best-effort
    /// in-memory counter; survives only the current process).
    pub panic_count: u32,
    /// Number of `notias.YYYY-MM-DD.log` files currently in `logs_dir`.
    pub log_files: u32,
    /// When the app process started (Unix epoch ms). `null` if unknown.
    pub started_at_ms: Option<i64>,
}

/// Reported to the frontend when the bootstrap phase failed (paths, db
/// open, integrity verify, or migrations). Rendered by `StartupErrorScreen`.
#[derive(Debug, Clone, Serialize)]
pub struct StartupError {
    /// Coarse category — same vocabulary as `AppError` codes.
    pub code: String,
    /// Phase in which the error happened (`"paths"`, `"db_open"`,
    /// `"db_verify"`, `"migrations"`, `"provider"`).
    pub phase: String,
    /// Human-readable message (already formatted via `Display`).
    pub message: String,
    /// Optional secondary detail (e.g. the migration that failed).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}