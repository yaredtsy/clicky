# Claw Kernel — macOS Accessibility Inspector

## What Are We Building?

An **Accessibility (A11y) Inspector** for macOS — a desktop tool built with **Tauri 2 + React + Swift** that:

1. **Extracts** the full accessibility tree of the frontmost macOS application
2. **Displays** that tree in a rich, interactive hierarchy (like a file/folder tree) in a React UI
3. **Enables live inspection** — when the user switches apps, the tree updates automatically
4. **[Future]** Allows mouse-based element selection (hover overlay like Xcode's Accessibility Inspector)
5. **[Future]** Allows clicking a tree node to highlight the corresponding element on the real app (like Grammarly's overlay)

---

## Why This Architecture? (Tauri + Swift + Rust)

| Layer | Technology | Responsibility | Why? |
|-------|-----------|----------------|------|
| **Native macOS** | Swift | Direct access to `AXUIElement` APIs, `NSWorkspace`, `NSPanel` overlays | Apple's Accessibility API is Objective-C/Swift only. No Rust bindings exist. |
| **Bridge** | `swift-rs` + Rust | Serialize AX data → JSON, expose Tauri commands | `swift-rs` lets us call Swift functions from Rust with zero Objective-C boilerplate. Rust gives us type safety and Tauri integration. |
| **Frontend** | React + TypeScript | Tree visualization, user interaction, state management | Fast iteration, rich ecosystem for tree components, familiar DX. |
| **Shell** | Tauri 2 | Window management, IPC, native menus, bundling | Lightweight alternative to Electron. Native webview, small binary. |

### Why Not Pure Rust?

macOS Accessibility APIs (`AXUIElement*`) are C/Objective-C APIs. While you *can* call them from Rust via `core-foundation` and raw FFI, it's extremely verbose and error-prone. Swift provides:
- First-class `AXUIElement` support
- `NSWorkspace` notifications for app activation
- `NSPanel` for transparent overlays (future feature)
- Memory management via ARC (vs manual `CFRetain`/`CFRelease`)

### Why Not Electron?

Tauri produces ~5MB binaries vs Electron's ~150MB. For a dev tool that inspects other apps, the lighter footprint matters. Tauri 2 also has better native plugin support.

---

## Current State of the Project

We already have a working foundation:

```
src-tauri/
├── swift/ClawAccessibility/     # Swift package — AX tree → XML serialization
├── src/lib.rs                    # Rust Tauri commands (bridge)
├── build.rs                      # swift-rs linker configuration
├── Info.plist                    # Accessibility usage description
└── tauri.conf.json               # Tauri app config

src/
├── App.tsx                       # React app — permission check + monitor start
├── App.css
└── main.tsx
```

### What Works Today
- ✅ Accessibility permission check and prompt
- ✅ Frontmost app detection via `NSWorkspace` observers
- ✅ Full AX tree serialization to XML file
- ✅ Event emission to React when frontmost app changes
- ✅ `swift-rs` bridge between Swift and Rust

### What We Need to Build (This Iteration)
- 🔲 Return AX tree as **JSON** (not just XML file) for React consumption
- 🔲 Define a proper **TypeScript model** for AX tree nodes
- 🔲 Build an **interactive tree viewer** in React
- 🔲 Add **node detail panel** showing all AX attributes
- 🔲 Refactor Swift layer into modular, scalable architecture
- 🔲 Refactor Rust bridge for clean command separation

---

## Iteration Roadmap

### Iteration 1 — AX Tree Extraction & Display (Current)
> **Goal**: Extract the accessibility tree and display it as an interactive hierarchy in React.

- Refactor Swift to return JSON instead of (or alongside) XML
- Define shared data model (Swift struct → JSON → TypeScript interface)
- Build tree viewer component with expand/collapse
- Show node attributes in a detail panel
- Live refresh on app switch

### Iteration 2 — Mouse Inspector & Overlay
> **Goal**: Let users hover over UI elements to inspect them, Xcode-style.

- Create transparent `NSPanel` overlay window
- Track mouse position globally via `CGEvent` tap
- Hit-test AX elements under cursor
- Draw bounding box overlay on hover
- Sync hovered element with tree selection

### Iteration 3 — Tree-to-App Highlight
> **Goal**: Click a node in the tree → highlight the element on the real app (Grammarly-style).

- Read `AXFrame` attribute for selected node
- Draw highlight rectangle on the overlay `NSPanel`
- Animate transitions between selected elements
- Support multi-select for parent chain visualization

---

## Key macOS Concepts You'll Learn

| Concept | Where You'll Learn It |
|---------|----------------------|
| `AXUIElement` and the Accessibility API | [01-macos-accessibility-fundamentals.md](./01-macos-accessibility-fundamentals.md) |
| Swift Package Manager + `swift-rs` bridge | [02-project-architecture.md](./02-project-architecture.md) |
| Tree traversal and element types | [03-swift-accessibility-layer.md](./03-swift-accessibility-layer.md) |
| Rust ↔ Swift FFI patterns | [04-rust-bridge-layer.md](./04-rust-bridge-layer.md) |
| React tree visualization | [05-react-tree-viewer.md](./05-react-tree-viewer.md) |
| Live sync and event architecture | [06-realtime-tree-sync.md](./06-realtime-tree-sync.md) |
| Mouse tracking and overlay windows | [07-future-mouse-inspector.md](./07-future-mouse-inspector.md) |
| Reverse highlight (tree → app) | [08-future-highlight-overlay.md](./08-future-highlight-overlay.md) |
| Testing and debugging AX code | [09-testing-and-debugging.md](./09-testing-and-debugging.md) |

---

## Prerequisites

- **macOS 10.15+** (Catalina or later)
- **Xcode Command Line Tools** (for Swift compiler)
- **Rust** (via `rustup`)
- **Node.js 18+** and `yarn`
- **Accessibility permission** granted to the built app (System Settings → Privacy & Security → Accessibility)

> **Important**: During development with `cargo tauri dev`, the Terminal (or IDE) that runs the process needs Accessibility permission, not the app itself. The bundled `.app` needs its own permission entry.
