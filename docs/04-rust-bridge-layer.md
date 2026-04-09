# 04 — Rust Bridge Layer (Step-by-Step)

> **Goal**: Refactor `lib.rs` into a modular Rust architecture that cleanly bridges Swift FFI ↔ Tauri commands.

---

## Table of Contents

1. [Current State → Target State](#1-current-state--target-state)
2. [Step 1: Create the Models Module](#step-1-create-the-models-module)
3. [Step 2: Create the Bridge Module](#step-2-create-the-bridge-module)
4. [Step 3: Create the State Module](#step-3-create-the-state-module)
5. [Step 4: Create the Commands Module](#step-4-create-the-commands-module)
6. [Step 5: Refactor lib.rs](#step-5-refactor-librs)
7. [Understanding swift-rs FFI in Detail](#understanding-swift-rs-ffi-in-detail)
8. [Error Handling Strategy](#error-handling-strategy)
9. [Serde Serialization Deep Dive](#serde-serialization-deep-dive)

---

## 1. Current State → Target State

### Current: Everything in `lib.rs` (193 lines)

```rust
// lib.rs contains:
// - Swift FFI declarations (swift!() macros)
// - Static state (OnceLock for AppHandle)
// - C callback function
// - All Tauri command handlers
// - App builder
```

### Target: Modular by Responsibility

```
src-tauri/src/
├── main.rs                  # Entry point (unchanged)
├── lib.rs                   # App builder + module declarations
├── commands/
│   ├── mod.rs               # Re-exports all commands
│   ├── accessibility.rs     # get_accessibility_tree
│   ├── permission.rs        # check/request permission
│   └── monitor.rs           # start/stop monitor
├── models/
│   ├── mod.rs               # Re-exports
│   └── ax_node.rs           # AXNode, Frame, AppInfo, AXTreeResponse
├── bridge/
│   ├── mod.rs               # Safe wrappers over FFI
│   └── swift_ffi.rs         # swift!() macro declarations (unsafe)
└── state/
    ├── mod.rs
    └── monitor_state.rs     # Global monitor handle
```

---

## Step 1: Create the Models Module

### `src-tauri/src/models/ax_node.rs`

```rust
use serde::{Deserialize, Serialize};

/// A single node in the macOS Accessibility tree.
///
/// This struct mirrors `AXNodeModel` in Swift and `AXNode` in TypeScript.
/// Serde handles the naming convention conversion:
/// - Rust: `child_count` (snake_case)
/// - JSON: `childCount` (camelCase)
/// - TypeScript: `childCount` (camelCase)
///
/// ## Why #[serde(rename_all = "camelCase")]?
///
/// JavaScript/TypeScript convention uses camelCase for object properties.
/// Rust convention uses snake_case. Rather than manually renaming each field,
/// serde does it automatically. This means the JSON produced by Swift's
/// `Codable` (which uses camelCase by default) matches this struct exactly.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AXNode {
    /// Unique ID within a single traversal (e.g., "n_0", "n_1").
    /// Not stable across traversals.
    pub id: String,

    /// Accessibility role: "AXButton", "AXWindow", "AXStaticText", etc.
    pub role: String,

    /// Optional subrole: "AXCloseButton", "AXSearchField", etc.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subrole: Option<String>,

    /// Element title (button text, window title, menu item label).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// Accessibility description for screen readers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Label value (less common, newer API).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,

    /// Help/tooltip text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,

    /// Stringified value (text content, checkbox state, slider value).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,

    /// Screen-coordinate frame (position + size).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame: Option<Frame>,

    /// Whether the element is enabled/interactive.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,

    /// Whether the element has keyboard focus.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focused: Option<bool>,

    /// Whether the element is selected (in lists/tables).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected: Option<bool>,

    /// Actions this element supports: ["AXPress", "AXShowMenu"].
    pub actions: Vec<String>,

    /// All attribute names this element supports.
    pub attributes: Vec<String>,

    /// Child nodes in the tree.
    pub children: Vec<AXNode>,

    /// Number of direct children.
    pub child_count: usize,
}

/// Screen-coordinate frame of an element.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frame {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Information about the inspected application.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub pid: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bundle_identifier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Top-level response containing app info + the full AX tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AXTreeResponse {
    pub app: AppInfo,
    pub root: AXNode,
    pub node_count: usize,
    pub truncated: bool,
}
```

### `src-tauri/src/models/mod.rs`

```rust
mod ax_node;

pub use ax_node::{AXNode, AXTreeResponse, AppInfo, Frame};
```

### Understanding `#[serde(skip_serializing_if)]`

```rust
#[serde(skip_serializing_if = "Option::is_none")]
pub title: Option<String>,
```

This means: if `title` is `None`, **omit it entirely** from the JSON output. Result:
```json
// WITH title:
{ "role": "AXButton", "title": "Save" }

// WITHOUT title (None):
{ "role": "AXButton" }  // "title" key is absent, not "title": null
```

**Why?** Reduces JSON payload size significantly. Most nodes don't have all attributes set.

---

## Step 2: Create the Bridge Module

### `src-tauri/src/bridge/swift_ffi.rs`

```rust
//! Raw Swift FFI declarations.
//!
//! This module contains the `swift!()` macro invocations that generate
//! `unsafe extern "C"` function bindings. These functions are defined in
//! Swift via `@_cdecl` in `FFI/Exports.swift`.
//!
//! ## Safety
//!
//! All functions in this module are inherently `unsafe` because they:
//! 1. Cross the FFI boundary (calling into a different language runtime)
//! 2. Assume the Swift side is compiled and linked correctly
//! 3. Assume the Swift functions exist with matching signatures
//!
//! This unsafety is contained within this module. The parent `bridge` module
//! exposes safe wrappers that validate inputs/outputs.

use std::ffi::c_void;
use swift_rs::{swift, Bool, SRString, SwiftRef};

// Permission
swift!(fn claw_ax_is_process_trusted(prompt: Bool) -> Bool);

// Tree extraction (JSON)
swift!(fn claw_ax_get_tree_json() -> SRString);

// File dump (XML)
swift!(fn claw_ax_dump_frontmost_to_file(path: &SRString) -> SRString);

// Monitor
swift!(fn claw_ax_start_frontmost_monitor(callback: *const c_void, dump_path: &SRString));
swift!(fn claw_ax_stop_frontmost_monitor());
```

### `src-tauri/src/bridge/mod.rs`

```rust
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
        swift_ffi::claw_ax_start_frontmost_monitor(
            callback as *const c_void,
            &path_sr,
        );
    }
}

/// Stop monitoring frontmost application changes.
pub fn stop_monitor() {
    unsafe { swift_ffi::claw_ax_stop_frontmost_monitor() }
}
```

### Why This Two-File Structure?

```
bridge/
├── swift_ffi.rs   # UNSAFE: raw FFI declarations
└── mod.rs         # SAFE: validated wrappers
```

**Separation of concerns for safety**:
- `swift_ffi.rs` is a declaration-only file. It generates the `unsafe extern "C"` functions.
- `mod.rs` wraps each one with validation (error string parsing, type conversion).
- `commands/` never uses `unsafe` — it only calls `bridge::` functions.

This pattern is called **safe FFI wrapper** and is standard practice in Rust projects.

---

## Step 3: Create the State Module

### `src-tauri/src/state/monitor_state.rs`

```rust
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
```

### `src-tauri/src/state/mod.rs`

```rust
mod monitor_state;

pub use monitor_state::{set_handle, with_handle};
```

### Why Not Tauri's Managed State?

Tauri has a `manage()` API for storing state:
```rust
app.manage(MyState::new());
```

We don't use it for the monitor handle because the monitor callback is a **C function pointer** — it can't receive a Tauri state reference. It needs to access global state directly.

For future features (cached trees, user preferences), we'll use Tauri managed state since those will be accessed from command handlers that receive `State<T>`.

---

## Step 4: Create the Commands Module

### `src-tauri/src/commands/permission.rs`

```rust
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
        log::info!("Accessibility permission check: {allowed}");
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
```

### `src-tauri/src/commands/accessibility.rs`

```rust
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
fn default_dump_path(app: &tauri::AppHandle) -> Result<String, String> {
    use tauri::Manager;
    let dir = app.path().document_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir
        .join("frontmost_ax_ui.xml")
        .to_string_lossy()
        .into_owned())
}
```

### `src-tauri/src/commands/monitor.rs`

```rust
//! Tauri commands for frontmost app monitoring.

use std::ffi::{c_char, CStr};
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

        let dump_path = super::accessibility::default_dump_path_from_handle(&app)?;
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
```

### `src-tauri/src/commands/mod.rs`

```rust
pub mod accessibility;
pub mod monitor;
pub mod permission;

// Re-export all commands for convenient registration in lib.rs
pub use accessibility::{get_accessibility_tree, dump_accessibility_tree_to_file};
pub use monitor::{start_accessibility_monitor, stop_accessibility_monitor};
pub use permission::{check_accessibility_permission, request_accessibility_permission};
```

---

## Step 5: Refactor `lib.rs`

### `src-tauri/src/lib.rs`

```rust
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
            commands::get_accessibility_tree,
            commands::dump_accessibility_tree_to_file,
            // Permission
            commands::check_accessibility_permission,
            commands::request_accessibility_permission,
            // Monitor
            commands::start_accessibility_monitor,
            commands::stop_accessibility_monitor,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

**Notice how clean this is.** The `run()` function is just configuration — no business logic, no `unsafe`, no FFI. Each command name is descriptive and grouped by domain.

---

## Understanding `swift-rs` FFI in Detail

### How `swift!()` Works

The `swift!()` macro generates an `unsafe extern "C"` function declaration:

```rust
// This:
swift!(fn claw_ax_is_process_trusted(prompt: Bool) -> Bool);

// Generates approximately this:
extern "C" {
    fn claw_ax_is_process_trusted(prompt: Bool) -> Bool;
}
```

At link time, the Rust binary links against the Swift static library produced by SPM. The `@_cdecl("claw_ax_is_process_trusted")` in Swift creates a C-compatible symbol that matches.

### `SRString` — String Across FFI

`swift-rs` provides `SRString` for passing strings between Rust and Swift:

```rust
// Rust → Swift: Convert &str to SRString
let path_sr: SRString = "/tmp/test.xml".into();
unsafe { claw_ax_dump_frontmost_to_file(&path_sr) };

// Swift → Rust: SRString return value
let result: SRString = unsafe { claw_ax_get_tree_json() };
let rust_string: &str = result.as_str();  // Zero-copy borrow
let owned: String = result.as_str().to_string();  // Allocating copy
```

### Build Configuration

The `build.rs` file tells the Rust compiler how to link the Swift library:

```rust
fn main() {
    #[cfg(target_os = "macos")]
    swift_rs::SwiftLinker::new("10.15")                    // Minimum macOS version
        .with_package("ClawAccessibility", "./swift/ClawAccessibility")  // SPM package
        .link();                                            // Generate linker flags
    tauri_build::build()
}
```

This:
1. Runs `swift build` on the SPM package
2. Generates linker flags to find the `.a` (static library)
3. Links the Swift runtime libraries

---

## Error Handling Strategy

### Convention: Result<T, String>

All Tauri commands return `Result<T, String>` where the `String` is a human-readable error message. This is sent to the frontend as a rejection:

```typescript
try {
  const tree = await invoke<AXTreeResponse>('get_accessibility_tree');
} catch (error) {
  // error is the String from Err(...)
  console.error('Failed:', error);
}
```

### Error Flow

```
Swift error
  → SRString starting with "error:"
    → bridge::get_tree_json() returns Err(message)
      → command returns Err(message)
        → Tauri IPC sends rejection
          → invoke() Promise rejects
            → React catch block
```

### Why Not Custom Error Types?

For V1, `String` errors are sufficient. The error messages are always displayed to the user in the UI. If we later need programmatic error handling (retry logic, specific UI for different errors), we'll introduce an error enum:

```rust
// Future improvement (not needed for V1)
#[derive(Debug, Serialize)]
pub enum AXError {
    PermissionDenied,
    NoFrontmostApp,
    TraversalFailed(String),
    SerializationFailed(String),
}
```

---

## Serde Serialization Deep Dive

### The Double Serialization Question

Data flows like this:
```
Swift Codable → JSON string → Rust serde → AXTreeResponse struct → Tauri serde → JSON to frontend
```

**Why not pass the raw JSON string through?**

We *could* return `String` from the command and parse in the frontend. But deserializing in Rust gives us:

1. **Validation**: If Swift produces malformed JSON, we catch it in Rust with a clear error
2. **Type safety**: The `AXTreeResponse` struct documents the exact shape
3. **Transformation**: We can add/modify fields before sending to frontend
4. **Future extensibility**: We might cache, diff, or filter the tree in Rust

The deserialization cost is negligible (~1ms for a 5,000-node tree).

### Matching Swift Codable ↔ Rust Serde

Swift's `Codable` produces property names in camelCase by default. Rust's `serde(rename_all = "camelCase")` consumes them. The mapping:

| Swift Property | JSON Key | Rust Field |
|---|---|---|
| `childCount` | `"childCount"` | `child_count` |
| `bundleIdentifier` | `"bundleIdentifier"` | `bundle_identifier` |
| `nodeCount` | `"nodeCount"` | `node_count` |

If these don't match, `serde_json::from_str` will fail with a descriptive error like:
```
missing field `childCount` at line 1 column 234
```

---

> **Next**: [05-react-tree-viewer.md](./05-react-tree-viewer.md) — Building the interactive tree UI in React.
