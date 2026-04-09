//! Safe wrappers around Swift FFI functions.
//!
//! This module is the **only place** in the Rust codebase that calls `unsafe`.
//! Command handlers in `commands/` call these safe functions instead of
//! touching FFI directly.

mod swift_ffi;

use std::ffi::c_void;
use swift_rs::SRString;

/// Check if this process has macOS Accessibility permission.
///
/// # Arguments
/// * `prompt` - If true, macOS may show the permission dialog.
pub fn is_process_trusted(prompt: bool) -> bool {
    unsafe { swift_ffi::claw_ax_is_process_trusted(prompt) }
}

/// Get the accessibility tree of the frontmost app as a JSON string.
///
/// # Returns
/// - `Ok(json_string)` on success
/// - `Err(error_message)` if the result starts with "error:"
pub fn get_tree_json() -> Result<String, String> {
    let result = unsafe { swift_ffi::claw_ax_get_tree_json() };
    let s = result.as_str().to_string();
    if s.starts_with("error:") {
        Err(s[6..].to_string()) // Strip "error:" prefix
    } else {
        Ok(s)
    }
}

/// Dump the frontmost app's AX tree to an XML file.
///
/// # Arguments
/// * `path` - Absolute file path to write to.
///
/// # Returns
/// - `Ok(path)` on success (the path that was written)
/// - `Err(error_message)` on failure
pub fn dump_to_file(path: &str) -> Result<String, String> {
    let path_sr: SRString = path.into();
    let result = unsafe { swift_ffi::claw_ax_dump_frontmost_to_file(&path_sr) };
    let s = result.as_str().to_string();
    if s.starts_with("error:") {
        Err(s[6..].to_string())
    } else if s.starts_with("ok:") {
        Ok(s[3..].to_string())
    } else {
        Ok(s)
    }
}

/// Start monitoring frontmost application changes.
///
/// # Safety contained
/// The callback pointer is cast from a Rust function pointer. This is safe
/// as long as the function has the correct signature (which it does, since
/// we control both sides).
pub fn start_monitor(callback: extern "C" fn(*const std::ffi::c_char), dump_path: &str) {
    let path_sr: SRString = dump_path.into();
    unsafe {
        swift_ffi::claw_ax_start_frontmost_monitor(callback as *const c_void, &path_sr);
    }
}

/// Stop monitoring frontmost application changes.
pub fn stop_monitor() {
    unsafe { swift_ffi::claw_ax_stop_frontmost_monitor() }
}
