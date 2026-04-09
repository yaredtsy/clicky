//! Global state for the frontmost app monitor.
//!
//! ## Why global state?
//!
//! The monitor callback is a C function pointer that Swift calls when the
//! frontmost app changes. C function pointers can't capture Rust closures,
//! so we need a global `AppHandle` that the callback can access.
//!
//! We use `OnceLock<Mutex<Option<AppHandle>>>`:
//! - `OnceLock`: Initialized once, accessible globally
//! - `Mutex`: Thread-safe access (callback may fire from any thread)
//! - `Option`: Handle may not be set yet

use std::sync::{Mutex, OnceLock};
use tauri::AppHandle;

static HANDLE: OnceLock<Mutex<Option<AppHandle>>> = OnceLock::new();

fn slot() -> &'static Mutex<Option<AppHandle>> {
    HANDLE.get_or_init(|| Mutex::new(None))
}

/// Store the AppHandle so the monitor callback can emit events.
pub fn set_handle(app: AppHandle) {
    *slot().lock().expect("monitor mutex poisoned") = Some(app);
}

/// Execute a closure with a reference to the stored AppHandle.
/// Returns None if no handle has been set.
pub fn with_handle<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&AppHandle) -> R,
{
    let guard = slot().lock().ok()?;
    guard.as_ref().map(f)
}
