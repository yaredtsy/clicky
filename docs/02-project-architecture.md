# 02 — Project Architecture & Folder Structure

> **Goal**: Define a scalable, maintainable folder structure for the Swift + Rust + React stack, with design patterns that support future features (mouse inspector, overlays).

---

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [Target Folder Structure](#2-target-folder-structure)
3. [Swift Layer Design](#3-swift-layer-design)
4. [Rust Bridge Layer Design](#4-rust-bridge-layer-design)
5. [React Frontend Design](#5-react-frontend-design)
6. [Data Flow Architecture](#6-data-flow-architecture)
7. [Design Patterns](#7-design-patterns)
8. [Why This Structure?](#8-why-this-structure)

---

## 1. Architecture Overview

```
┌────────────────────────────────────────────────────────────┐
│                    React Frontend (UI)                      │
│  ┌──────────┐  ┌───────────┐  ┌──────────────────────────┐ │
│  │ Tree     │  │ Detail    │  │ Tools (future: inspector) │ │
│  │ Viewer   │  │ Panel     │  │                          │ │
│  └────┬─────┘  └─────┬─────┘  └────────┬─────────────────┘ │
│       │              │                  │                   │
│  ┌────┴──────────────┴──────────────────┴────────────┐     │
│  │          Tauri IPC (invoke / events / listen)      │     │
│  └───────────────────────┬───────────────────────────┘     │
└──────────────────────────┼─────────────────────────────────┘
                           │ Tauri Commands
┌──────────────────────────┼─────────────────────────────────┐
│                    Rust Bridge Layer                        │
│  ┌───────────┐  ┌────────┴───────┐  ┌──────────────┐     │
│  │ commands/ │  │ models/        │  │ state/        │     │
│  │ (IPC)     │  │ (shared types) │  │ (app state)   │     │
│  └─────┬─────┘  └───────────────┘  └──────────────┘     │
│        │ FFI via swift-rs                                  │
│  ┌─────┴──────────────────────────────────────────────┐   │
│  │              Swift ↔ Rust FFI boundary              │   │
│  └─────┬──────────────────────────────────────────────┘   │
└────────┼──────────────────────────────────────────────────┘
         │
┌────────┼──────────────────────────────────────────────────┐
│        │         Swift Native Layer                        │
│  ┌─────┴──────┐  ┌────────────┐  ┌──────────────────────┐│
│  │ Extraction │  │ Monitoring │  │ Overlay (future)      ││
│  │ (AX tree)  │  │ (app focus) │  │ (NSPanel, drawing)   ││
│  └────────────┘  └────────────┘  └──────────────────────┘│
│                                                            │
│  macOS Accessibility API (ApplicationServices)             │
└────────────────────────────────────────────────────────────┘
```

---

## 2. Target Folder Structure

Here is the complete target structure. Files marked with `[NEW]` need to be created; files marked `[EXISTS]` already exist and may need refactoring.

```
claw-kernel/
├── docs/                                # ← You are here
│   ├── 00-overview.md
│   ├── 01-macos-accessibility-fundamentals.md
│   ├── 02-project-architecture.md
│   └── ...
│
├── src/                                 # React frontend
│   ├── main.tsx                         # [EXISTS] Entry point
│   ├── App.tsx                          # [EXISTS] Root component → refactor
│   ├── App.css                          # [EXISTS] Global styles → refactor
│   │
│   ├── components/                      # [NEW] Reusable UI components
│   │   ├── tree/                        # Tree viewer components
│   │   │   ├── TreeView.tsx             # Root tree container
│   │   │   ├── TreeNode.tsx             # Single expandable node
│   │   │   ├── TreeNode.css             # Node-level styles
│   │   │   └── index.ts                 # Barrel export
│   │   │
│   │   ├── detail/                      # Node detail panel
│   │   │   ├── DetailPanel.tsx          # Attribute display panel
│   │   │   ├── DetailPanel.css
│   │   │   └── index.ts
│   │   │
│   │   ├── toolbar/                     # App toolbar / controls
│   │   │   ├── Toolbar.tsx
│   │   │   ├── Toolbar.css
│   │   │   └── index.ts
│   │   │
│   │   └── permission/                  # Permission gate UI
│   │       ├── PermissionGate.tsx
│   │       └── index.ts
│   │
│   ├── hooks/                           # [NEW] Custom React hooks
│   │   ├── useAccessibilityTree.ts      # AX tree fetching + state
│   │   ├── usePermission.ts             # Permission check hook
│   │   ├── useFrontmostApp.ts           # Listen for app changes
│   │   └── useSelectedNode.ts           # Tree selection state
│   │
│   ├── models/                          # [NEW] TypeScript types
│   │   ├── ax-tree.ts                   # AXNode, AXTree interfaces
│   │   ├── app-info.ts                  # FrontmostApp info type
│   │   └── index.ts
│   │
│   ├── services/                        # [NEW] Tauri IPC wrappers
│   │   ├── accessibility.ts             # invoke() wrappers for AX commands
│   │   └── events.ts                    # Event listener helpers
│   │
│   └── utils/                           # [NEW] Utility functions
│       ├── tree-utils.ts                # Tree search, flatten, filter
│       └── format.ts                    # Display formatting helpers
│
├── src-tauri/
│   ├── Cargo.toml                       # [EXISTS] Dependencies
│   ├── build.rs                         # [EXISTS] Swift linker config
│   ├── Info.plist                       # [EXISTS] Permissions
│   ├── tauri.conf.json                  # [EXISTS] Tauri config
│   │
│   ├── src/                             # Rust source
│   │   ├── main.rs                      # [EXISTS] Entry point
│   │   ├── lib.rs                       # [EXISTS] → refactor into modules
│   │   │
│   │   ├── commands/                    # [NEW] Tauri command handlers
│   │   │   ├── mod.rs                   # Re-exports
│   │   │   ├── accessibility.rs         # AX tree commands
│   │   │   ├── permission.rs            # Permission commands
│   │   │   └── monitor.rs              # Frontmost app monitor commands
│   │   │
│   │   ├── models/                      # [NEW] Shared data structures
│   │   │   ├── mod.rs
│   │   │   ├── ax_node.rs               # AXNode struct (Serialize)
│   │   │   └── app_info.rs              # AppInfo struct
│   │   │
│   │   ├── bridge/                      # [NEW] Swift FFI declarations
│   │   │   ├── mod.rs
│   │   │   └── swift_ffi.rs             # swift!() macro declarations
│   │   │
│   │   └── state/                       # [NEW] App state management
│   │       ├── mod.rs
│   │       └── monitor_state.rs         # Monitor handle + state
│   │
│   └── swift/
│       └── ClawAccessibility/
│           ├── Package.swift            # [EXISTS] SPM manifest
│           └── Sources/
│               └── ClawAccessibility/
│                   ├── ClawAccessibility.swift  # [EXISTS] → refactor
│                   │
│                   │   # Target refactored structure:
│                   ├── Core/                    # [NEW] Core AX utilities
│                   │   ├── AXHelpers.swift       # axString, axBool, axFrame
│                   │   ├── AXTraversal.swift     # Tree traversal logic
│                   │   └── AXPermission.swift    # Permission checking
│                   │
│                   ├── Models/                  # [NEW] Data models
│                   │   └── AXNodeModel.swift     # Codable node struct
│                   │
│                   ├── Serialization/           # [NEW] Output formats
│                   │   ├── JSONSerializer.swift   # Tree → JSON
│                   │   └── XMLSerializer.swift    # Tree → XML (existing)
│                   │
│                   ├── Monitor/                 # [NEW] App monitoring
│                   │   └── FrontmostMonitor.swift # NSWorkspace observers
│                   │
│                   ├── FFI/                     # [NEW] Rust-facing exports
│                   │   └── Exports.swift         # @_cdecl functions
│                   │
│                   └── Overlay/                 # [FUTURE] Inspector overlay
│                       ├── OverlayPanel.swift    # NSPanel subclass
│                       ├── OverlayView.swift     # Drawing layer
│                       └── HitTesting.swift      # Mouse → AX element
```

---

## 3. Swift Layer Design

### Single Responsibility Principle

The current `ClawAccessibility.swift` (305 lines) handles everything: XML generation, AX helpers, permission checks, monitoring, FFI exports. We need to split by responsibility.

### Module Breakdown

#### `Core/AXHelpers.swift` — Low-Level AX Utilities
```swift
// Pure utility functions for reading AX attributes
// No side effects, no state, easily testable

func axString(_ element: AXUIElement, _ attr: String) -> String?
func axBool(_ element: AXUIElement, _ attr: String) -> Bool?
func axInt(_ element: AXUIElement, _ attr: String) -> Int?
func axFrame(_ element: AXUIElement) -> CGRect?
func axChildren(_ element: AXUIElement) -> [AXUIElement]
func axActionNames(_ element: AXUIElement) -> [String]
func axAttributeNames(_ element: AXUIElement) -> [String]
```

**Why separate?** These are the foundational building blocks. Every other module depends on them. Isolating them makes testing and reuse trivial.

#### `Core/AXTraversal.swift` — Tree Walking
```swift
// Tree traversal with cycle detection and limits
// Returns a structured model, NOT serialized output

struct TraversalConfig {
    let maxDepth: Int       // Default: 120
    let maxNodes: Int       // Default: 60_000
    let attributes: [String] // Which attributes to fetch
}

func traverseTree(
    root: AXUIElement,
    config: TraversalConfig
) -> AXNodeModel
```

**Why separate from serialization?** Traversal produces a **model**. Serialization converts that model to JSON or XML. This separation means we can:
- Add new output formats without touching traversal
- Cache the model and serialize later
- Filter/transform the model before serialization

#### `Core/AXPermission.swift` — Permission Logic
```swift
func isProcessTrusted(prompt: Bool) -> Bool
```

Tiny but important to isolate — future iterations may add more sophisticated permission flow.

#### `Models/AXNodeModel.swift` — Data Model
```swift
/// Represents a single node in the accessibility tree.
/// Codable for automatic JSON serialization via Foundation.
struct AXNodeModel: Codable {
    let id: String            // Unique ID for this traversal (UUID or pointer hash)
    let role: String          // "AXButton", "AXWindow", etc.
    let subrole: String?      // "AXCloseButton", etc.
    let title: String?
    let description: String?
    let label: String?
    let help: String?
    let value: String?        // Stringified value
    let frame: FrameModel?    // Position + size
    let enabled: Bool?
    let focused: Bool?
    let selected: Bool?
    let actions: [String]
    let attributes: [String]  // All available attribute names
    let children: [AXNodeModel]
    let childCount: Int       // Useful for lazy loading UI
}

struct FrameModel: Codable {
    let x: Double
    let y: Double
    let width: Double
    let height: Double
}
```

**Why a dedicated model?** 
- `AXUIElement` cannot be passed across FFI (it's a remote reference)
- We need a serializable snapshot of the tree at a moment in time
- The model is the **contract** between Swift and TypeScript

#### `Serialization/JSONSerializer.swift`
```swift
func serializeToJSON(root: AXNodeModel) -> String {
    let encoder = JSONEncoder()
    encoder.outputFormatting = [.sortedKeys]  // Not prettyPrinted for perf
    let data = try! encoder.encode(root)
    return String(data: data, encoding: .utf8)!
}
```

#### `Serialization/XMLSerializer.swift`
Refactored version of the current XML serialization logic, now working on `AXNodeModel` instead of directly on `AXUIElement`.

#### `Monitor/FrontmostMonitor.swift`
Extracted `NSWorkspace` observer logic — `startMonitoring()`, `stopMonitoring()`, callback handling.

#### `FFI/Exports.swift`
All `@_cdecl` functions in one place — the **only** file that talks to Rust:
```swift
@_cdecl("claw_ax_get_tree_json")
public func claw_ax_get_tree_json() -> SRString { ... }

@_cdecl("claw_ax_is_process_trusted")
public func claw_ax_is_process_trusted(prompt: Bool) -> Bool { ... }

@_cdecl("claw_ax_start_frontmost_monitor")
public func claw_ax_start_frontmost_monitor(...) { ... }

@_cdecl("claw_ax_stop_frontmost_monitor")  
public func claw_ax_stop_frontmost_monitor() { ... }
```

**Why centralize FFI exports?** 
- Single place to audit the Rust ↔ Swift boundary
- Clear naming convention (`claw_ax_*`)
- Easy to add new exports without hunting through files

---

## 4. Rust Bridge Layer Design

### Module Breakdown

#### `commands/` — Tauri Command Handlers

Each file maps to a domain. Commands are thin wrappers that:
1. Call Swift FFI
2. Deserialize the result
3. Return to the frontend

```rust
// commands/accessibility.rs
#[tauri::command]
fn get_accessibility_tree(app: AppHandle) -> Result<AXNode, String> {
    let json = bridge::get_tree_json()?;
    let node: AXNode = serde_json::from_str(&json)
        .map_err(|e| format!("Failed to parse AX tree: {e}"))?;
    Ok(node)
}
```

**Why thin commands?** Commands should only do IPC concerns (argument validation, error mapping, state access). Business logic stays in Swift or in dedicated Rust modules.

#### `models/` — Shared Data Structures

```rust
// models/ax_node.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AXNode {
    pub id: String,
    pub role: String,
    pub subrole: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub label: Option<String>,
    pub help: Option<String>,
    pub value: Option<String>,
    pub frame: Option<Frame>,
    pub enabled: Option<bool>,
    pub focused: Option<bool>,
    pub selected: Option<bool>,
    pub actions: Vec<String>,
    pub attributes: Vec<String>,
    pub children: Vec<AXNode>,
    pub child_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frame {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
```

**`#[serde(rename_all = "camelCase")]`**: Converts Rust's `snake_case` to JavaScript's `camelCase` automatically. The TypeScript interface uses `childCount`, Rust uses `child_count`, serde handles the conversion.

#### `bridge/` — Swift FFI Declarations

```rust
// bridge/swift_ffi.rs
use swift_rs::{swift, Bool, SRString};

swift!(fn claw_ax_get_tree_json() -> SRString);
swift!(fn claw_ax_is_process_trusted(prompt: Bool) -> Bool);
swift!(fn claw_ax_start_frontmost_monitor(callback: *const c_void, dump_path: &SRString));
swift!(fn claw_ax_stop_frontmost_monitor());
```

**Why isolate FFI?** 
- The `swift!()` macro generates `unsafe extern "C"` functions. Keeping them in one module makes the unsafe boundary explicit and auditable.
- `commands/` never touches `unsafe` directly — it calls `bridge::` functions that handle the unsafe wrapping.

#### `state/` — Application State

```rust
// state/monitor_state.rs
use std::sync::{Mutex, OnceLock};
use tauri::AppHandle;

static HANDLE: OnceLock<Mutex<Option<AppHandle>>> = OnceLock::new();

pub fn set_handle(app: AppHandle) { ... }
pub fn with_handle<F, R>(f: F) -> Option<R> where F: FnOnce(&AppHandle) -> R { ... }
```

---

## 5. React Frontend Design

### Component Hierarchy

```
App
├── PermissionGate              # Shows permission status, blocks if not allowed
│   └── (children only render if permission is granted)
│
├── Toolbar                     # Controls: refresh, start/stop monitor, search
│   ├── AppSelector             # Shows current frontmost app info
│   ├── RefreshButton
│   └── SearchInput (future)
│
├── SplitPane                   # Resizable horizontal split
│   ├── TreePanel               # Left panel
│   │   └── TreeView
│   │       ├── TreeNode (recursive)
│   │       ├── TreeNode
│   │       │   ├── TreeNode
│   │       │   └── TreeNode
│   │       └── TreeNode
│   │
│   └── DetailPanel             # Right panel
│       ├── AttributeTable      # Key-value attribute display
│       ├── ActionList           # Available actions
│       └── FrameDisplay        # Visual frame representation
```

### State Management Strategy

We use **React hooks** (no external state library) because:
- The state is simple and local (one tree, one selection)
- No complex derived state or cross-component communication
- Custom hooks provide clean encapsulation

```typescript
// hooks/useAccessibilityTree.ts
export function useAccessibilityTree() {
  const [tree, setTree] = useState<AXNode | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  
  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const result = await invoke<AXNode>("get_accessibility_tree");
      setTree(result);
      setError(null);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  return { tree, loading, error, refresh };
}
```

---

## 6. Data Flow Architecture

### Request Flow (User Clicks Refresh)

```
React                    Rust                     Swift
  │                       │                         │
  │ invoke("get_ax_tree") │                         │
  │──────────────────────>│                         │
  │                       │ claw_ax_get_tree_json() │
  │                       │────────────────────────>│
  │                       │                         │ 1. NSWorkspace.frontmostApplication
  │                       │                         │ 2. AXUIElementCreateApplication(pid)
  │                       │                         │ 3. traverseTree(root)
  │                       │                         │ 4. JSONEncoder.encode(model)
  │                       │        JSON string      │
  │                       │<────────────────────────│
  │                       │ serde_json::from_str()  │
  │     AXNode (JSON)     │                         │
  │<──────────────────────│                         │
  │                       │                         │
  │ setState(tree)        │                         │
  │ render TreeView       │                         │
```

### Event Flow (App Switch Detected)

```
macOS                    Swift                    Rust                    React
  │                       │                        │                       │
  │ didActivateApp        │                        │                       │
  │──────────────────────>│                        │                       │
  │                       │ 1. traverseTree()      │                       │
  │                       │ 2. Encode JSON         │                       │
  │                       │ callback(bundleId)      │                       │
  │                       │───────────────────────>│                        │
  │                       │                        │ app.emit("ax-tree-    │
  │                       │                        │   updated", json)     │
  │                       │                        │──────────────────────>│
  │                       │                        │                       │ setState
  │                       │                        │                       │ re-render
```

### Why JSON Over XML?

| Aspect | XML (current) | JSON (target) |
|--------|--------------|---------------|
| **Parsing in React** | Need an XML parser | `JSON.parse()` — native, fast |
| **Type safety** | Manual extraction | TypeScript interfaces map directly |
| **Bundle size** | Need xml2js or similar | Zero dependency |
| **Serialization** | Manual string building | `Codable` in Swift, `serde` in Rust |
| **Nesting** | Verbose with tags | Clean object hierarchy |

We'll keep XML as an optional export format (file dump) but use JSON as the primary IPC format.

---

## 7. Design Patterns

### Pattern 1: Facade Pattern (Swift FFI Exports)

`FFI/Exports.swift` acts as a **facade** — a simplified interface to a complex subsystem:

```swift
// External API (simple)
@_cdecl("claw_ax_get_tree_json")
public func claw_ax_get_tree_json() -> SRString { ... }

// Internal complexity (hidden)
// → AXPermission.isProcessTrusted()
// → NSWorkspace.frontmostApplication
// → AXUIElementCreateApplication()
// → AXTraversal.traverseTree()
// → JSONSerializer.serialize()
```

**Benefit**: Rust only knows about 4-5 functions. It doesn't know about `AXUIElement`, `NSWorkspace`, or any Apple framework detail.

### Pattern 2: Bridge Pattern (Rust ↔ Swift)

The `bridge/` module in Rust abstracts the FFI boundary:

```rust
// Public safe API
pub fn get_tree_json() -> Result<String, String> {
    let result = unsafe { claw_ax_get_tree_json() };
    let s = result.as_str().to_string();
    if s.starts_with("error:") {
        Err(s)
    } else {
        Ok(s)
    }
}
```

**Benefit**: `commands/` never deals with `unsafe`. The unsafe boundary is confined to `bridge/`.

### Pattern 3: Command Pattern (Tauri Commands)

Each Tauri command is a discrete operation with clear input/output:

```rust
#[tauri::command]
fn get_accessibility_tree() -> Result<AXNode, String> { ... }

#[tauri::command]  
fn check_permission() -> Result<bool, String> { ... }
```

**Benefit**: Easy to test, document, and evolve independently.

### Pattern 4: Observer Pattern (Event System)

For real-time updates, we use Tauri's event system (which uses the Observer pattern under the hood):

```
Publisher: Swift (NSWorkspace observer) 
    → Rust (Tauri event emitter) 
        → Subscriber: React (event listener)
```

### Pattern 5: Container/Presenter (React Components)

```
Container (hook):  useAccessibilityTree()    ← data fetching, state
Presenter:         <TreeView tree={tree} />  ← pure rendering
```

**Benefit**: Tree rendering is decoupled from data source. We can swap the data source (live AX, cached, mock) without changing the UI components.

---

## 8. Why This Structure?

### Scalability Vectors

| Future Feature | What Changes | What Stays Untouched |
|---------------|-------------|---------------------|
| Mouse inspector | Add `Overlay/` in Swift, new commands in Rust, new panel in React | `Core/`, `Models/`, `TreeView`, existing commands |
| Lazy loading | Modify `AXTraversal.swift` to support partial traversal, add expand command | `JSONSerializer`, `FFI/Exports` signature, React hooks |
| Search | Add `tree-utils.ts` filter logic | `TreeView` (add highlight prop), Swift layer |
| Multi-window | Modify traversal to handle multiple windows | `Models/` (add window grouping), `DetailPanel` |
| Export to file | Add new serializer (CSV? HTML?) | `Core/`, `Monitoring/`, React layer |

### Naming Conventions

| Layer | Convention | Example |
|-------|-----------|---------|
| Swift files | PascalCase | `AXHelpers.swift`, `JSONSerializer.swift` |
| Swift functions | camelCase | `traverseTree()`, `isProcessTrusted()` |
| Swift FFI exports | snake_case with `claw_ax_` prefix | `claw_ax_get_tree_json` |
| Rust modules | snake_case | `ax_node.rs`, `swift_ffi.rs` |
| Rust commands | snake_case | `get_accessibility_tree` |
| TypeScript files | kebab-case | `ax-tree.ts`, `tree-utils.ts` |
| TypeScript types | PascalCase | `AXNode`, `Frame`, `AppInfo` |
| React components | PascalCase | `TreeView.tsx`, `DetailPanel.tsx` |
| CSS files | Component-matched | `TreeNode.css`, `DetailPanel.css` |

### Dependency Direction (Strict)

```
React → Tauri IPC → Rust Commands → Rust Bridge → Swift FFI → Swift Core
  ↓                                                              ↓
models/     ← shared types (TS interfaces mirror Rust structs) → Models/
```

**Rule**: Dependencies only flow **right and down**. Swift `Core/` never imports `FFI/`. React `components/` never imports Tauri directly (goes through `services/`).

---

> **Next**: [03-swift-accessibility-layer.md](./03-swift-accessibility-layer.md) — Step-by-step implementation of the Swift layer.
