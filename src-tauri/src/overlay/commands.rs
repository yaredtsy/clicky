//! Tauri commands for the highlight overlay.

use crate::models::ax_node::Frame;

/// Highlight a UI element using the overlay panel at AX screen coordinates.
#[tauri::command]
pub fn highlight_element(app: tauri::AppHandle, frame: Frame) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        super::panel::show_highlight(&app, frame)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (app, frame);
        Err("Highlight overlay is only available on macOS.".into())
    }
}

/// Hide the highlight overlay.
#[tauri::command]
pub fn clear_highlight(app: tauri::AppHandle) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        super::panel::hide_highlight(&app)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = app;
        Err("Highlight overlay is only available on macOS.".into())
    }
}
