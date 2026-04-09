mod bridge;
mod commands;
mod models;
mod overlay;
mod state;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default().plugin(tauri_plugin_opener::init());

    #[cfg(target_os = "macos")]
    {
        builder = builder.plugin(tauri_nspanel::init());
    }

    builder
        .invoke_handler(tauri::generate_handler![
            // Accessibility tree
            commands::accessibility::get_accessibility_tree,
            commands::accessibility::dump_accessibility_tree_to_file,
            // Permission
            commands::permission::check_accessibility_permission,
            commands::permission::request_accessibility_permission,
            // Monitor
            commands::monitor::start_accessibility_monitor,
            commands::monitor::stop_accessibility_monitor,
            // Highlight overlay
            overlay::commands::highlight_element,
            overlay::commands::clear_highlight,
        ])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            {
                if let Err(e) = overlay::panel::create_overlay_panel(app.handle()) {
                    eprintln!("[claw-kernel] Failed to create highlight overlay panel: {e}");
                }
                if let Err(e) = overlay::panel::create_tooltip_panel(app.handle()) {
                    eprintln!("[claw-kernel] Failed to create highlight tooltip panel: {e}");
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
