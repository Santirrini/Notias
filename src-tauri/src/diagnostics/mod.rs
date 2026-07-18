//! Diagnostics — observability primitives for the Notias desktop app.
//!
//! Exposes three responsibilities:
//!
//! * [`log_setup::init`] — install a `tracing` subscriber that writes to a
//!   daily-rotated file under `<data_dir>/logs/`. Replaces the previous
//!   stdout-only subscriber.
//! * [`panic_hook::install`] — install a `std::panic::set_hook` that writes
//!   a `crash.log` file with the panic message, location, and a forced
//!   backtrace. Without this, a panic in release Windows would just abort
//!   silently because `windows_subsystem` suppresses the console.
//! * [`commands`] — Tauri IPC commands the frontend uses to render the
//!   Diagnostics panel and to export a zipped support bundle.
//!
//! Designed to be 100% additive: nothing in this module affects existing
//! command behaviour, schema, or wire format.

pub mod commands;
pub mod log_setup;
pub mod panic_hook;
pub mod types;

pub use types::{DiagnosticsSnapshot, LogEntry, StartupError};