//! Tauri commands for the highlight overlay.

use crate::models::ax_node::Frame;

/// Highlight a UI element using the overlay panel at AX screen coordinates.
///
/// IPC payload must use **one JSON key per parameter** (Tauri 2): `frame`, `title`, `description`.
/// A single nested `args` object is not used.
#[tauri::command]
pub fn highlight_element(
    app: tauri::AppHandle,
    frame: Frame,
    title: Option<String>,
    description: Option<String>,
) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        super::panel::show_highlight(&app, frame, title, description)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (app, frame, title, description);
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
