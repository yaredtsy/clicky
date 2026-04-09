//! Tauri commands for Accessibility permission management.

/// Check current Accessibility permission WITHOUT showing a prompt.
///
/// ## When to use
/// - On app startup to show permission status in the UI
/// - After the user claims they've granted permission (re-check)
///
/// ## Frontend usage
/// ```typescript
/// const allowed = await invoke<boolean>('check_accessibility_permission');
/// ```
#[tauri::command]
pub fn check_accessibility_permission() -> Result<bool, String> {
    #[cfg(target_os = "macos")]
    {
        let allowed = crate::bridge::is_process_trusted(false);
        Ok(allowed)
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err("Accessibility is only available on macOS.".into())
    }
}

/// Request Accessibility permission (shows system prompt on first call).
///
/// ## Behavior
/// - First call: macOS shows a dialog directing user to System Settings
/// - Subsequent calls: Silently returns current trust state
///
/// ## Returns
/// - `true` if already trusted
/// - `false` if not trusted (but prompt was shown)
///
/// ## Frontend usage
/// ```typescript
/// const alreadyTrusted = await invoke<boolean>('request_accessibility_permission');
/// if (!alreadyTrusted) {
///   showMessage("Grant permission in System Settings, then click Refresh");
/// }
/// ```
#[tauri::command]
pub fn request_accessibility_permission() -> Result<bool, String> {
    #[cfg(target_os = "macos")]
    {
        Ok(crate::bridge::is_process_trusted(true))
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err("Accessibility is only available on macOS.".into())
    }
}
