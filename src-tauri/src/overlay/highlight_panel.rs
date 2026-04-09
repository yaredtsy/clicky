//! `tauri_panel!` must live in its own module — it imports objc2 types that clash with ours.

use tauri::Manager;
use tauri_nspanel::tauri_panel;

tauri_panel! {
    panel!(HighlightOverlay {
        config: {
            can_become_key_window: false,
            can_become_main_window: false,
            is_floating_panel: true,
            hides_on_deactivate: false
        }
    })
}
