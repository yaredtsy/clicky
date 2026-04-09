mod bridge;
mod commands;
mod models;
mod state;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
