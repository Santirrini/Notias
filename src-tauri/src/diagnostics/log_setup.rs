//! Install the application-wide `tracing` subscriber that writes to a
//! daily-rotated log file under `<data_dir>/logs/`.
//!
//! Replaces the previous stdout-only subscriber in `lib.rs::run`. On
//! Windows release builds the stdout/stderr is suppressed by
//! `windows_subsystem`, so without a file target every log line would be
//! lost — this module fixes that.
//!
//! The subscriber is process-global. Tests that need a different writer
//! should use `tracing::subscriber::with_default` locally instead of
//! calling `init()`.

use crate::db::AppPaths;
use crate::error::AppResult;
use std::path::{Path, PathBuf};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::RollingFileAppender;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};

/// Guard returned by `init` so the worker thread isn't dropped when
/// `init` returns (which would silently flush-loss). Held by `AppState`'s
/// surrounding `Mutex` is overkill; instead we stash it in a `OnceLock`
/// process-global slot owned by this module.
static GUARD: std::sync::OnceLock<WorkerGuard> = std::sync::OnceLock::new();

/// Path to today's log file under `logs_dir`. Exposed for `commands.rs`
/// (which needs to read the latest file to tail recent entries).
pub fn current_log_file(paths: &AppPaths) -> PathBuf {
    paths.logs_dir.join("notias.log")
}

/// Initialize the global subscriber.
///
/// Idempotent: calling `init` twice in the same process returns `Ok(())`
/// without re-installing the subscriber — `try_init` is a no-op after the
/// first successful install, and the worker guard is kept alive for the
/// lifetime of the process via `GUARD`.
pub fn init(paths: &AppPaths) -> AppResult<()> {
    init_with_dir(&paths.logs_dir)
}

/// Test-friendly variant — takes the directory directly.
pub fn init_with_dir(logs_dir: &Path) -> AppResult<()> {
    std::fs::create_dir_all(logs_dir)?;

    // Daily rotation. Filename pattern is configurable; we mirror Tauri-
    // style logs (`notias.YYYY-MM-DD`). The rolling appender auto-rotates
    // at midnight local time.
    let appender: RollingFileAppender = tracing_appender::rolling::daily(logs_dir, "notias.log");

    // `non_blocking` wraps the file so the writer thread doesn't block
    // instrumentation. The guard must outlive the program — we hand it to
    // `GUARD` so it lives until process exit.
    let (file_writer, guard) = tracing_appender::non_blocking(appender);
    let _ = GUARD.set(guard); // ignore "already set" — that's the idempotent path

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("notias_lib=info,warn"));

    let fmt_layer = fmt::layer()
        .with_writer(file_writer)
        .with_target(true)
        .with_ansi(false) // ANSI escapes would corrupt the file
        .with_level(true);

    let subscriber = Registry::default().with(env_filter).with(fmt_layer);

    // `try_init` returns Err if a global subscriber is already installed.
    // That's fine for tests — we keep the guard alive either way.
    let _ = subscriber.try_init();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use tracing::{info, warn};

    #[test]
    fn init_writes_log_file_and_contains_messages() {
        let dir = tempdir().unwrap();
        init_with_dir(dir.path()).expect("init");

        info!("hello-info");
        warn!("hello-warn");

        // Force a flush: drop the file writer to commit the buffer.
        // We can't reach the worker guard from here, so instead we look
        // for the current-day log file directly.
        let expected = dir.path().join("notias.log");
        // The rolling daily appender writes to `notias.YYYY-MM-DD.log`;
        // on the same day the "current" file is that one. Search for any
        // matching prefix.
        let entries = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("notias") && n.ends_with(".log"))
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>();
        assert!(
            !entries.is_empty(),
            "expected a notias*.log file under {}",
            dir.path().display()
        );

        // At least one file should contain our markers. Read them all and
        // concatenate — small test, the cost is negligible.
        let combined: String = entries
            .iter()
            .filter_map(|p| std::fs::read_to_string(p).ok())
            .collect();
        assert!(combined.contains("hello-info"), "missing info marker");
        assert!(combined.contains("hello-warn"), "missing warn marker");
    }

    #[test]
    fn init_creates_missing_logs_dir() {
        let dir = tempdir().unwrap();
        let nested = dir.path().join("a").join("b");
        assert!(!nested.exists());
        init_with_dir(&nested).expect("init");
        assert!(nested.is_dir());
    }
}