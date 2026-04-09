//! Tauri commands for frontmost app monitoring.

use std::ffi::{CStr, c_char};
use tauri::{AppHandle, Emitter};

/// Start monitoring frontmost app changes.
///
/// When the user switches to a different app, the monitor:
/// 1. Traverses the new app's AX tree
/// 2. Writes XML to the dump file
/// 3. Emits "ax-frontmost-changed" event with the bundle ID
///
/// ## Prerequisites
/// - Accessibility permission must be granted
///
/// ## Frontend usage
/// ```typescript
/// await invoke('start_accessibility_monitor');
/// // Listen for changes:
/// const unlisten = await listen<string>('ax-frontmost-changed', (e) => {
///   console.log('Switched to:', e.payload);
/// });
/// ```
#[tauri::command]
pub fn start_accessibility_monitor(app: AppHandle) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        if !crate::bridge::is_process_trusted(true) {
            return Err(
                "Accessibility permission required. Enable in System Settings → Privacy & Security → Accessibility."
                    .into(),
            );
        }

        let dump_path = super::accessibility::default_dump_path(&app)?;
        crate::state::set_handle(app);
        crate::bridge::start_monitor(frontmost_changed_callback, &dump_path);
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = app;
        Err("App monitoring is only available on macOS.".into())
    }
}

/// Stop monitoring frontmost app changes.
#[tauri::command]
pub fn stop_accessibility_monitor() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        crate::bridge::stop_monitor();
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err("App monitoring is only available on macOS.".into())
    }
}

/// C callback invoked by Swift when the frontmost app changes.
///
/// This function is passed as a C function pointer to Swift. It receives
/// the bundle identifier (or "pid:<N>") as a null-terminated C string.
///
/// ## Why extern "C"?
/// Swift's `@convention(c)` expects a C-compatible function pointer.
/// Rust closures can't be used here because they have an unknown size
/// and may capture state. `extern "C" fn` has a known ABI.
#[cfg(target_os = "macos")]
extern "C" fn frontmost_changed_callback(bundle_c: *const c_char) {
    if bundle_c.is_null() {
        return;
    }
    let bundle_id = unsafe { CStr::from_ptr(bundle_c) }
        .to_string_lossy()
        .into_owned();

    crate::state::with_handle(|app| {
        let _ = app.emit("ax-frontmost-changed", &bundle_id);
    });
}
