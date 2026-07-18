//! Install a global panic hook that writes a `crash.log` file.
//!
//! Background: on Windows release builds, `windows_subsystem = "windows"`
//! in `main.rs` suppresses the console, and the default panic hook writes
//! to stderr. Result: panics in production are invisible. This module
//! gives panics a durable on-disk destination so the user can find them
//! via the Diagnostics panel and the support bundle.
//!
//! The hook is process-global and **overwrites** the previous hook
//! intentionally — we want to own it for the lifetime of the app so the
//! default stderr-writing behavior never fires.

use crate::diagnostics::types::{LogLevel, LogEntry};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Once;

/// File name for crash dumps — separate from rolling logs because it
/// persists across rotations and we want at most one entry per panic.
pub const CRASH_LOG: &str = "crash.log";

/// Process-local counter of panics captured since startup. Surfaced by
/// the diagnostics snapshot.
static PANIC_COUNT: AtomicU32 = AtomicU32::new(0);
static INSTALLED: Once = Once::new();

/// Returns the number of panics captured so far in this process.
pub fn panic_count() -> u32 {
    PANIC_COUNT.load(Ordering::Relaxed)
}

/// Path to the crash log (`<logs_dir>/crash.log`). Convenience for the
/// snapshot builder.
pub fn crash_log_path(logs_dir: &Path) -> PathBuf {
    logs_dir.join(CRASH_LOG)
}

/// Install the global panic hook. Idempotent — only the first call has
/// effect. Best-effort: if the logs dir cannot be created, the hook
/// silently no-ops and the default behavior takes over (which still
/// emits a console message in dev builds).
pub fn install(logs_dir: &Path) {
    let _ = std::fs::create_dir_all(logs_dir);

    // Pre-format the path so the hook closure can clone it cheaply.
    let path = logs_dir.join(CRASH_LOG);

    INSTALLED.call_once(|| {
        let default_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            PANIC_COUNT.fetch_add(1, Ordering::Relaxed);

            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0);

            // Best-effort capture; Backtrace is forced on regardless of
            // the env-var because we explicitly want it in the log.
            let backtrace = std::backtrace::Backtrace::force_capture();
            let entry = LogEntry {
                ts_ms: now,
                level: LogLevel::Error,
                target: "panic".to_string(),
                message: format!(
                    "panic: {}\nlocation: {}\nbacktrace:\n{}",
                    info,
                    info.location()
                        .map(|l| format!("{}:{}", l.file(), l.line()))
                        .unwrap_or_else(|| "<unknown>".into()),
                    backtrace
                ),
            };

            // Append to crash.log. We open-write-close each time so a
            // truncated previous write doesn't truncate a new one — and
            // we never hold the file open across panics.
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
            {
                let line = format!(
                    "[{}] [{}] [{}] {}\n",
                    entry.ts_ms,
                    entry.level.as_str(),
                    entry.target,
                    entry.message.replace('\n', "\\n")
                );
                let _ = f.write_all(line.as_bytes());
            }

            // Delegate to the previous hook so dev builds still get a
            // nice console message and IDE breakpoint behaviour.
            default_hook(info);
        }));
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::{catch_unwind, AssertUnwindSafe};
    use tempfile::tempdir;

    #[test]
    fn install_writes_crash_log_on_panic() {
        let dir = tempdir().unwrap();
        let logs = dir.path().to_path_buf();
        install(&logs);

        let result = catch_unwind(AssertUnwindSafe(|| {
            panic!("intentional test panic");
        }));
        assert!(result.is_err(), "panic should have been caught");

        let path = crash_log_path(&logs);
        assert!(path.exists(), "crash.log not created at {:?}", path);

        let contents = std::fs::read_to_string(&path).unwrap();
        assert!(contents.contains("intentional test panic"));
        assert!(contents.contains("backtrace:"));
    }

    #[test]
    fn panic_count_increments() {
        let dir = tempdir().unwrap();
        install(&dir.path().to_path_buf());

        let before = panic_count();
        let _ = catch_unwind(AssertUnwindSafe(|| {
            panic!("another test panic");
        }));
        let after = panic_count();
        assert!(after > before, "panic_count should have increased");
    }

    #[test]
    fn install_is_idempotent() {
        let dir = tempdir().unwrap();
        // Calling twice must not panic or change behaviour.
        install(&dir.path().to_path_buf());
        install(&dir.path().to_path_buf());
    }
}