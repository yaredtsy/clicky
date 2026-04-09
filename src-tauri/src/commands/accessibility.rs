//! Tauri commands for Accessibility tree extraction.

use crate::models::AXTreeResponse;

/// Get the accessibility tree of the frontmost macOS app.
///
/// Returns the full tree as a structured JSON object that the frontend
/// can directly render as a tree view.
///
/// ## Flow
/// 1. Swift gets the frontmost app via NSWorkspace
/// 2. Creates AXUIElement for the app's PID
/// 3. Recursively traverses the AX tree (DFS, with limits)
/// 4. Serializes to JSON via Codable
/// 5. Rust deserializes JSON → AXTreeResponse struct
/// 6. Tauri serializes struct → JSON response to frontend
///
/// Note: The double JSON round-trip (Swift JSON → Rust struct → Tauri JSON)
/// is intentional. It validates the data at the Rust layer and allows us to
/// transform/enrich the data before sending to the frontend.
///
/// ## Frontend usage
/// ```typescript
/// const tree = await invoke<AXTreeResponse>('get_accessibility_tree');
/// // tree.root is the root AXNode
/// // tree.app has { pid, bundleIdentifier, name }
/// ```
#[tauri::command]
pub fn get_accessibility_tree() -> Result<AXTreeResponse, String> {
    #[cfg(target_os = "macos")]
    {
        let json = crate::bridge::get_tree_json()?;
        let response: AXTreeResponse = serde_json::from_str(&json)
            .map_err(|e| format!("Failed to parse AX tree JSON: {e}"))?;
        Ok(response)
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err("Accessibility tree is only available on macOS.".into())
    }
}

/// Dump the frontmost app's AX tree to an XML file.
///
/// If `path` is None or empty, writes to ~/Documents/frontmost_ax_ui.xml.
///
/// ## Frontend usage
/// ```typescript
/// const filePath = await invoke<string>('dump_accessibility_tree_to_file', {
///   path: '/tmp/tree.xml' // optional
/// });
/// ```
#[tauri::command]
pub fn dump_accessibility_tree_to_file(
    app: tauri::AppHandle,
    path: Option<String>,
) -> Result<String, String> {
    #[cfg(target_os = "macos")]
    {
        let p = match path {
            Some(ref p) if !p.is_empty() => p.clone(),
            _ => default_dump_path(&app)?,
        };
        crate::bridge::dump_to_file(&p)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (app, path);
        Err("XML dump is only available on macOS.".into())
    }
}

/// Resolve the default file path for AX dumps.
#[cfg(target_os = "macos")]
pub(super) fn default_dump_path(app: &tauri::AppHandle) -> Result<String, String> {
    use tauri::Manager;
    let dir = app.path().document_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir
        .join("frontmost_ax_ui.xml")
        .to_string_lossy()
        .into_owned())
}
