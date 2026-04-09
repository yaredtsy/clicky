# 07 — Tree-to-App Highlight (Select Node → Highlight on Screen)

> **Goal**: When the user clicks a node in the tree viewer, draw a highlight rectangle over the corresponding real UI element on the target app — like Xcode's Accessibility Inspector or Grammarly's overlay.

---

## Table of Contents

1. [How It Works (High-Level)](#1-how-it-works-high-level)
2. [Architecture Overview](#2-architecture-overview)
3. [Step 1: Add `tauri-nspanel` Dependency](#step-1-add-tauri-nspanel-dependency)
4. [Step 2: Define the Overlay Panel Class (Rust)](#step-2-define-the-overlay-panel-class-rust)
5. [Step 3: Create the Overlay Panel on App Start](#step-3-create-the-overlay-panel-on-app-start)
6. [Step 4: Create the Overlay Panel HTML/CSS](#step-4-create-the-overlay-panel-htmlcss)
7. [Step 5: Create the Highlight Command (Rust)](#step-5-create-the-highlight-command-rust)
8. [Step 6: Wire the React Tree Selection → Highlight](#step-6-wire-the-react-tree-selection--highlight)
9. [Coordinate System Deep Dive](#coordinate-system-deep-dive)
10. [Why NSPanel and Not a Regular NSWindow?](#why-nspanel-and-not-a-regular-nswindow)
11. [Edge Cases and Gotchas](#edge-cases-and-gotchas)
12. [Future Enhancements](#future-enhancements)
13. [Files Changed Summary](#files-changed-summary)

---

## 1. How It Works (High-Level)

```
User clicks "AXButton '7'" in tree
        │
        ▼
React reads node.frame = { x: 340, y: 410, w: 52, h: 40 }
        │
        ▼
invoke("highlight_element", { frame })
        │
        ▼
Rust moves + resizes the overlay NSPanel to those coordinates
        │
        ▼
Overlay NSPanel draws a translucent blue rectangle
over the Calculator's "7" button
        │
        ▼
User sees the real button highlighted on screen
```

The key insight: **we don't draw into the target app**. We create a transparent, always-on-top `NSPanel` that sits *above* all other windows. We position and resize it to exactly match the AX element's frame. Because the panel is transparent except for a border/fill, it appears as a highlight rectangle floating over the target app.

---

## 2. Architecture Overview

```
┌── React (main window) ────────────────────────────┐
│                                                    │
│  TreeView: user clicks node with frame={...}       │
│      │                                             │
│      └─ invoke("highlight_element", { frame })     │
│                                                    │
└────────────────────┬───────────────────────────────┘
                     │ Tauri IPC
                     ▼
┌── Rust ────────────────────────────────────────────┐
│                                                    │
│  commands/overlay.rs:                              │
│    1. Get the overlay panel handle                 │
│    2. Set panel frame to match the AX frame        │
│    3. Show the panel                               │
│                                                    │
└────────────────────┬───────────────────────────────┘
                     │ NSPanel API via tauri-nspanel
                     ▼
┌── macOS ───────────────────────────────────────────┐
│                                                    │
│  Overlay NSPanel (transparent, borderless)          │
│  ┌──────────────────────────────────────────────┐  │
│  │ overlay.html:                                │  │
│  │  <div> with blue border + translucent fill   │  │
│  │  Positioned at {x, y, width, height}         │  │
│  └──────────────────────────────────────────────┘  │
│                                                    │
│  Target App (e.g., Calculator)                     │
│  ┌──────────────────────────────────────────────┐  │
│  │  The real UI underneath the overlay           │  │
│  └──────────────────────────────────────────────┘  │
│                                                    │
└────────────────────────────────────────────────────┘
```

---

## Step 1: Add `tauri-nspanel` Dependency

### `src-tauri/Cargo.toml`

```toml
[dependencies]
tauri = { version = "2", features = [] }
tauri-plugin-opener = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# NSPanel for overlay highlight windows
tauri-nspanel = { git = "https://github.com/ahkohd/tauri-nspanel", branch = "v2.1" }

[target.'cfg(target_os = "macos")'.dependencies]
swift-rs = "1.0.7"
```

### Why `v2.1` Branch?

The `v2.1` branch is the latest and supports:
- Tauri v2 (which we're using)
- `PanelBuilder` API (fluent, ergonomic panel creation)
- `tauri_panel!` macro (define panel classes with custom behavior)
- Mouse tracking events (for the reverse direction in doc 08)
- Thread-safe operations handled on the main thread automatically

---

## Step 2: Define the Overlay Panel Class (Rust)

### `src-tauri/src/overlay/mod.rs`

```rust
pub mod panel;
pub mod commands;
```

### `src-tauri/src/overlay/panel.rs`

```rust
//! Overlay panel definition and lifecycle.
//!
//! The overlay panel is a transparent, borderless NSPanel that floats
//! above all other windows. It renders a single highlighted rectangle
//! via a small HTML page loaded from overlay.html.
//!
//! ## Why NSPanel?
//!
//! NSPanel is a subclass of NSWindow with special behaviors:
//! - Can float above other apps without activating our app
//! - Can be non-activating (clicking it doesn't steal focus)
//! - Works across Spaces (all desktops)
//! - Supports `ignoresMouseEvents` (clicks pass through to app below)
//!
//! This is exactly what Xcode's Accessibility Inspector and Grammarly use.

use tauri::Manager;
use tauri_nspanel::{
    tauri_panel, CollectionBehavior, PanelBuilder, PanelLevel,
    StyleMask, WebviewUrl, ManagerExt,
};

// Define our overlay panel class with the tauri_panel! macro.
//
// Config explained:
// - can_become_key_window: false   → Panel never steals keyboard focus
// - can_become_main_window: false  → Panel never becomes the "main" window
// - is_floating_panel: true        → Floats above regular windows
// - hides_on_deactivate: false     → Stays visible when our app loses focus
//                                    (critical: we need it visible when the
//                                     user is looking at the target app)
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

/// Label for the overlay panel window. Used to retrieve it later.
pub const OVERLAY_LABEL: &str = "highlight-overlay";

/// Create the overlay panel. Call once during app setup.
///
/// The panel is created hidden — it will be shown when the user
/// selects a node in the tree viewer.
pub fn create_overlay_panel(app: &tauri::AppHandle) -> Result<(), String> {
    let panel = PanelBuilder::<_, HighlightOverlay>::new(app, OVERLAY_LABEL)
        // Load the overlay HTML (a simple page with a highlight div)
        .url(WebviewUrl::App("overlay.html".into()))
        // Transparent background so only the highlight rectangle is visible
        .transparent(true)
        // No shadow (we're just a highlight rectangle)
        .has_shadow(false)
        // Float above everything, including other apps
        .level(PanelLevel::ScreenSaver)
        // Don't steal focus from the user's current app
        .no_activate(true)
        // Non-activating panel style = clicking doesn't activate our app
        .style_mask(
            StyleMask::empty()
                .borderless()
                .nonactivating_panel()
        )
        // Show on all Spaces, skip Cmd+Tab, don't move with Spaces
        .collection_behavior(
            CollectionBehavior::new()
                .can_join_all_spaces()
                .stationary()
                .ignores_cycle()
                .full_screen_auxiliary()
        )
        // Configure the underlying Tauri window
        .with_window(|window| {
            window
                .decorations(false)       // No title bar
                .skip_taskbar(true)       // Don't show in taskbar/dock
                .resizable(false)         // We control size programmatically
                .always_on_top(true)
        })
        .build()
        .map_err(|e| format!("Failed to create overlay panel: {e}"))?;

    // Click-through: mouse events pass through to the app below.
    // Without this, clicking the highlighted area would click our panel
    // instead of the target app's button.
    panel.set_ignores_mouse_events(true);

    // Start hidden — shown when user selects a node
    panel.hide();

    Ok(())
}

/// Show the overlay panel positioned at the given screen frame.
///
/// ## Coordinate System
///
/// The `frame` uses AX screen coordinates:
/// - Origin: top-left of primary display
/// - Y increases downward
///
/// NSPanel uses AppKit coordinates:
/// - Origin: bottom-left of primary display
/// - Y increases upward
///
/// We must convert between them (see coordinate section below).
pub fn show_highlight(
    app: &tauri::AppHandle,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), String> {
    let panel = app
        .get_webview_panel(OVERLAY_LABEL)
        .map_err(|e| format!("Overlay panel not found: {e}"))?;

    // Convert AX coordinates (top-left origin) to AppKit (bottom-left origin)
    let screen_height = get_primary_screen_height();
    let appkit_y = screen_height - y - height;

    // Move and resize the panel to match the element's frame
    let ns_panel = panel.as_panel();
    unsafe {
        use objc2_foundation::NSRect;
        use objc2_foundation::NSPoint;
        use objc2_foundation::NSSize;

        let frame = NSRect::new(
            NSPoint::new(x, appkit_y),
            NSSize::new(width, height),
        );
        // `false` = don't animate the frame change (instant repositioning)
        ns_panel.setFrame_display_(frame, false);
    }

    // Show the panel
    panel.order_front_regardless();

    // Tell the overlay page to update its display
    // (in case we want to show element info in the overlay later)
    if let Some(window) = panel.to_window() {
        let _ = window.emit("highlight-update", serde_json::json!({
            "x": x, "y": y, "width": width, "height": height
        }));
    }

    Ok(())
}

/// Hide the overlay panel (e.g., when selection is cleared).
pub fn hide_highlight(app: &tauri::AppHandle) -> Result<(), String> {
    let panel = app
        .get_webview_panel(OVERLAY_LABEL)
        .map_err(|e| format!("Overlay panel not found: {e}"))?;
    panel.hide();
    Ok(())
}

/// Get the height of the primary screen in points.
///
/// We need this for coordinate conversion between AX (top-left origin)
/// and AppKit (bottom-left origin).
fn get_primary_screen_height() -> f64 {
    #[cfg(target_os = "macos")]
    {
        use objc2_app_kit::NSScreen;
        unsafe {
            if let Some(screen) = NSScreen::mainScreen() {
                return screen.frame().size.height;
            }
        }
    }
    // Fallback
    1080.0
}
```

### Understanding the Panel Configuration

| Config | Value | Why |
|--------|-------|-----|
| `can_become_key_window` | `false` | Panel never receives keyboard focus. User continues typing in their current app. |
| `can_become_main_window` | `false` | Panel never becomes the "main" window. Prevents our app from activating when the overlay shows. |
| `is_floating_panel` | `true` | Floats above regular windows. The highlight must be visible on top of the target app. |
| `hides_on_deactivate` | `false` | **Critical**: Stays visible when our app loses focus. Without this, the overlay disappears when the user clicks on the target app. |
| `ignoresMouseEvents` | `true` | Clicks pass through the overlay to the app below. The user can still interact with the highlighted button. |
| `PanelLevel::ScreenSaver` | Highest level | Ensures the overlay is above everything, including other floating panels. |
| `nonactivating_panel` | Style mask | Prevents the panel from activating our app. |
| `can_join_all_spaces` | Collection | Overlay follows the user across all macOS Spaces/desktops. |
| `ignores_cycle` | Collection | Overlay doesn't appear in Cmd+Tab. |

---

## Step 3: Create the Overlay Panel on App Start

### Update `src-tauri/src/lib.rs`

```rust
mod bridge;
mod commands;
mod models;
mod overlay;   // NEW
mod state;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_nspanel::init())   // Register NSPanel plugin
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
            // Overlay (NEW)
            overlay::commands::highlight_element,
            overlay::commands::clear_highlight,
        ])
        .setup(|app| {
            // Create the overlay panel during app startup
            #[cfg(target_os = "macos")]
            overlay::panel::create_overlay_panel(app.handle())
                .expect("Failed to create overlay panel");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

---

## Step 4: Create the Overlay Panel HTML/CSS

This is the content rendered *inside* the overlay NSPanel. It's a minimal HTML page that draws a highlight rectangle.

### `src/overlay.html` (or `public/overlay.html`)

> Put this in the same directory your `index.html` is served from. Since Tauri serves from `frontendDist`, place it alongside your built output, or in the `public/` directory so Vite copies it as-is.

```html
<!DOCTYPE html>
<html>
<head>
  <meta charset="UTF-8" />
  <title>Highlight Overlay</title>
  <style>
    /* The entire page IS the highlight rectangle.
       The NSPanel is sized to match the AX element's frame,
       so this div fills the panel exactly. */
    * {
      margin: 0;
      padding: 0;
      box-sizing: border-box;
    }
    html, body {
      width: 100%;
      height: 100%;
      overflow: hidden;
      background: transparent;
      /* Allow clicks to pass through */
      pointer-events: none;
    }
    .highlight {
      width: 100%;
      height: 100%;
      /* Semi-transparent blue fill */
      background: rgba(59, 130, 246, 0.12);
      /* Solid blue border */
      border: 2px solid rgba(59, 130, 246, 0.85);
      border-radius: 3px;
      /* Subtle animation when appearing */
      animation: highlight-appear 0.15s ease-out;
    }
    @keyframes highlight-appear {
      from {
        opacity: 0;
        transform: scale(1.05);
      }
      to {
        opacity: 1;
        transform: scale(1);
      }
    }

    /* Optional: element info tooltip at the top */
    .info-badge {
      position: absolute;
      top: -22px;
      left: 0;
      background: rgba(30, 30, 46, 0.95);
      color: #89dceb;
      font-family: 'SF Mono', 'Menlo', monospace;
      font-size: 11px;
      padding: 2px 6px;
      border-radius: 3px;
      white-space: nowrap;
      pointer-events: none;
      border: 1px solid rgba(59, 130, 246, 0.5);
    }
  </style>
</head>
<body>
  <div class="highlight" id="highlight"></div>
  <script>
    // Listen for highlight updates from Rust (future: show element info)
    if (window.__TAURI__) {
      window.__TAURI__.event.listen('highlight-update', (event) => {
        // Could add a tooltip badge showing the role/title
        // For now, the rectangle itself is sufficient
        console.log('Highlight update:', event.payload);
      });
    }
  </script>
</body>
</html>
```

### Why a Webview for the Overlay?

You might wonder: why use a webview (HTML/CSS) for a simple rectangle? Couldn't we draw it natively?

**Reasons for webview approach**:
1. **`tauri-nspanel` creates webview panels** — it's what the library provides. Using it means zero custom Objective-C.
2. **Future flexibility**: We'll want to show element info, parent chain, tooltips. HTML/CSS is easier for these rich displays than `NSView` drawing.
3. **Theming**: CSS makes it trivial to change the highlight color, animation, style.
4. **Simplicity**: The alternative is creating a custom `NSView` with `draw(_:)` override in Swift — more code, harder to iterate on.

**Downsides**:
- A webview for a rectangle is "heavy" (~5MB memory). Acceptable for a dev tool.
- Slight startup latency. We create it once and reuse it, so it's fine.

---

## Step 5: Create the Highlight Command (Rust)

### `src-tauri/src/overlay/commands.rs`

```rust
//! Tauri commands for the highlight overlay.

use crate::models::Frame;

/// Highlight a UI element by drawing the overlay at its frame coordinates.
///
/// Called when the user clicks a node in the tree that has a frame.
/// If the node has no frame (some elements don't), the frontend should
/// not call this command.
///
/// ## Frontend Usage
/// ```typescript
/// if (selectedNode.frame) {
///   await invoke('highlight_element', { frame: selectedNode.frame });
/// }
/// ```
#[tauri::command]
pub fn highlight_element(app: tauri::AppHandle, frame: Frame) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        super::panel::show_highlight(
            &app,
            frame.x,
            frame.y,
            frame.width,
            frame.height,
        )
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (app, frame);
        Err("Highlight overlay is only available on macOS.".into())
    }
}

/// Clear the highlight overlay (hide it).
///
/// Called when:
/// - The user deselects a node
/// - The user selects a node without a frame
/// - The tree is refreshed (frames change position)
///
/// ## Frontend Usage
/// ```typescript
/// await invoke('clear_highlight');
/// ```
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
```

---

## Step 6: Wire the React Tree Selection → Highlight

### Update `src/services/accessibility.ts`

```typescript
import type { Frame } from '../models';

// ... existing exports ...

/** Show the highlight overlay at the given frame. */
export async function highlightElement(frame: Frame): Promise<void> {
  return invoke<void>('highlight_element', { frame });
}

/** Hide the highlight overlay. */
export async function clearHighlight(): Promise<void> {
  return invoke<void>('clear_highlight');
}
```

### Update `src/hooks/useSelectedNode.ts`

```typescript
import { useCallback, useEffect, useState } from 'react';
import type { AXNode } from '../models';
import { highlightElement, clearHighlight } from '../services/accessibility';

interface UseSelectedNodeResult {
  selectedNode: AXNode | null;
  selectedId: string | null;
  select: (node: AXNode) => void;
  deselect: () => void;
}

export function useSelectedNode(): UseSelectedNodeResult {
  const [selectedNode, setSelectedNode] = useState<AXNode | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);

  // When selection changes, update the overlay
  useEffect(() => {
    if (selectedNode?.frame) {
      void highlightElement(selectedNode.frame);
    } else {
      void clearHighlight();
    }
  }, [selectedNode]);

  const select = useCallback((node: AXNode) => {
    setSelectedNode(node);
    setSelectedId(node.id);
  }, []);

  const deselect = useCallback(() => {
    setSelectedNode(null);
    setSelectedId(null);
  }, []);

  return { selectedNode, selectedId, select, deselect };
}
```

### What Happens When the User Clicks a Tree Node

```
1. User clicks "AXButton '7'" in TreeNode component
2. TreeNode calls onSelect(node)
3. useSelectedNode.select(node) sets state
4. useEffect fires: node has frame → highlightElement(frame)
5. invoke("highlight_element", { frame: { x: 340, y: 410, w: 52, h: 40 } })
6. Rust moves the overlay NSPanel to (340, 410) with size (52, 40)
7. The panel appears over the Calculator's "7" button
8. User sees a blue highlight rectangle on the real button
```

---

## Coordinate System Deep Dive

This is the trickiest part of the whole feature. macOS has **two different coordinate systems** and we must convert between them.

### AX Coordinates (What We Get from the AX API)

```
Primary Display:
  ┌─────────────────────────────┐
  │ (0, 0)            (1920, 0) │  ← Origin at TOP-LEFT
  │                             │
  │   AXButton frame:           │
  │   x=340, y=410              │     Y increases DOWNWARD
  │   ┌──────┐                  │
  │   │  7   │ h=40             │
  │   └──────┘ w=52             │
  │                             │
  │ (0, 1080)        (1920,1080)│
  └─────────────────────────────┘
```

### AppKit Coordinates (What NSPanel Uses)

```
Primary Display:
  ┌─────────────────────────────┐
  │ (0, 1080)        (1920,1080)│  ← TOP has max Y
  │                             │
  │   NSPanel frame needs:      │
  │   x=340, y=630              │     Y increases UPWARD
  │   ┌──────┐                  │
  │   │  7   │ h=40             │
  │   └──────┘ w=52             │
  │                             │
  │ (0, 0)            (1920, 0) │  ← Origin at BOTTOM-LEFT
  └─────────────────────────────┘
```

### The Conversion Formula

```
appkit_y = screenHeight - ax_y - elementHeight
```

In our example:
```
appkit_y = 1080 - 410 - 40 = 630
```

### Multi-Display Gotcha

With multiple displays, screen coordinates extend beyond a single display:

```
┌───────────────┐┌─────────────────────────────┐
│   Secondary   ││        Primary              │
│   Display     ││        Display              │
│               ││                             │
│ x: -1920..0   ││ x: 0..1920                  │
│ y: depends on ││ y: 0..1080                  │
│ arrangement   ││                             │
└───────────────┘└─────────────────────────────┘
```

The AX frame coordinates work across all displays — an element on the secondary display might have `x = -800`. The conversion still works because we use the **primary** screen height for the Y-flip, and `NSPanel` handles negative X coordinates correctly.

### Getting the Primary Screen Height

```rust
fn get_primary_screen_height() -> f64 {
    #[cfg(target_os = "macos")]
    unsafe {
        // NSScreen.mainScreen returns the screen containing
        // the key window, which may not be the primary display.
        // For coordinate conversion, we need the primary
        // display's height. NSScreen.screens().first() is always
        // the primary display.
        use objc2_app_kit::NSScreen;
        if let Some(screens) = NSScreen::screens() {
            if let Some(primary) = screens.first() {
                return primary.frame().size.height;
            }
        }
    }
    1080.0  // Safe fallback
}
```

> **Note**: `NSScreen.mainScreen()` returns the screen with the key window, NOT the primary display. For coordinate conversion, always use `NSScreen.screens().first()` which is guaranteed to be the primary.

---

## Why NSPanel and Not a Regular NSWindow?

| Feature | NSWindow | NSPanel |
|---------|----------|---------|
| Float without activating app | ❌ Activates app when shown | ✅ `nonactivating_panel` style mask |
| Stay visible when our app is background | ❌ Hidden by default | ✅ `hides_on_deactivate: false` |
| Skip Cmd+Tab | ❌ Appears in app switcher | ✅ `ignores_cycle` collection behavior |
| Click-through | ❌ Captures all clicks | ✅ `ignoresMouseEvents = true` |
| Utility window styling | ❌ Full-size title bar | ✅ Utility window style (smaller/no title bar) |

`NSPanel` is literally designed for exactly this use case — auxiliary floating UI that doesn't interfere with the user's workflow.

---

## Edge Cases and Gotchas

### 1. Element With No Frame

Some AX elements don't have a `frame` attribute (e.g., menu bar items when the menu isn't open, some invisible elements):

```typescript
// In the selection handler, check for frame
if (selectedNode.frame) {
  await highlightElement(selectedNode.frame);
} else {
  await clearHighlight();
}
```

### 2. Element Moved or Resized After Traversal

The AX tree is a **snapshot**. If the user resizes a window or scrolls, the real element's position changes but our stored frame is stale. Solutions:
- **V1**: Live with it. Clear highlight on tree refresh.
- **V2**: Re-query the specific element's frame when highlighting (requires keeping an AX element reference, which we don't have after serialization).
- **V3**: Use `AXObserver` to watch for position changes and update in real-time.

### 3. Overlay Blocks Menu Bar

If an AX element's frame overlaps the menu bar (y < 25, typically), the overlay might cover it. Use a minimum Y offset:

```rust
let safe_y = appkit_y.max(0.0);  // Don't extend below screen bottom
```

### 4. Zero-Size Frames

Some elements have zero width or height. Don't show the overlay for these:

```rust
if frame.width < 1.0 || frame.height < 1.0 {
    return hide_highlight(app);
}
```

### 5. When to Clear the Highlight

Clear the overlay when:
- Tree is refreshed (positions changed)
- User deselects node
- Frontmost app changes (the old positions are meaningless)
- User selects a node without a frame

```typescript
// In useAccessibilityTree.ts, clear highlight on refresh
const refresh = useCallback(async () => {
  await clearHighlight();   // Clear before fetching new tree
  setLoading(true);
  // ... fetch tree
}, []);
```

---

## Future Enhancements

### Info Badge Tooltip

Show the element's role and title above the highlight:

```html
<div class="info-badge" id="badge">AXButton "7"</div>
```

Update via Tauri event:
```javascript
listen('highlight-update', (e) => {
  document.getElementById('badge').textContent = 
    `${e.payload.role} ${e.payload.title ? '"' + e.payload.title + '"' : ''}`;
});
```

### Parent Chain Highlight

When selecting a node, also draw faded outlines around its parent, grandparent, etc. This helps understand nesting:

```
┌─────────────────────────────────────┐  AXWindow (faint outline)
│ ┌─────────────────────────────────┐ │  AXGroup (medium outline)
│ │ ┌─────┐                        │ │
│ │ │  7  │ ← AXButton (solid)     │ │
│ │ └─────┘                        │ │
│ └─────────────────────────────────┘ │
└─────────────────────────────────────┘
```

### Animated Transitions

Animate the overlay smoothly between different elements:

```rust
// setFrame_display_animate_
ns_panel.setFrame_display_animate_(frame, true, true);
```

Or use CSS transitions in `overlay.html` by resizing the outer panel but keeping the inner div transition smooth.

---

## Files Changed Summary

| File | Change | Status |
|------|--------|--------|
| `src-tauri/Cargo.toml` | Add `tauri-nspanel` dependency | MODIFY |
| `src-tauri/src/lib.rs` | Register NSPanel plugin, setup overlay, register commands | MODIFY |
| `src-tauri/src/overlay/mod.rs` | New module | NEW |
| `src-tauri/src/overlay/panel.rs` | Panel definition, create/show/hide | NEW |
| `src-tauri/src/overlay/commands.rs` | `highlight_element`, `clear_highlight` Tauri commands | NEW |
| `public/overlay.html` | Overlay panel HTML/CSS content | NEW |
| `src/services/accessibility.ts` | Add `highlightElement()`, `clearHighlight()` wrappers | MODIFY |
| `src/hooks/useSelectedNode.ts` | Trigger highlight on selection change | MODIFY |

---

> **Next**: [08-mouse-to-tree-inspector.md](./08-mouse-to-tree-inspector.md) — The reverse direction: hover over an element on screen → select it in the tree.
