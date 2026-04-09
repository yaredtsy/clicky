# 01 — macOS Accessibility Fundamentals

> **Goal**: Understand the macOS Accessibility API from the ground up. This is the foundation everything else builds on.

---

## Table of Contents

1. [What is the Accessibility API?](#1-what-is-the-accessibility-api)
2. [The AXUIElement Model](#2-the-axuielement-model)
3. [Permission Model](#3-permission-model)
4. [Roles — What Types of Elements Exist?](#4-roles--what-types-of-elements-exist)
5. [Attributes — What Data Can You Read?](#5-attributes--what-data-can-you-read)
6. [Actions — What Can You Trigger?](#6-actions--what-can-you-trigger)
7. [The Element Hierarchy](#7-the-element-hierarchy)
8. [Getting the Frontmost Application](#8-getting-the-frontmost-application)
9. [Traversing Children](#9-traversing-children)
10. [Common Pitfalls and Edge Cases](#10-common-pitfalls-and-edge-cases)
11. [Performance Considerations](#11-performance-considerations)

---

## 1. What is the Accessibility API?

macOS provides the **Accessibility API** (part of `ApplicationServices` framework) to allow assistive technologies (screen readers, automation tools, inspectors) to:

- **Read** the UI structure of any running application
- **Query** attributes of any UI element (label, role, frame, state)
- **Perform** actions on elements (press buttons, set focus, scroll)
- **Observe** changes via notifications

The API is **process-external** — you can inspect *other* apps' UI trees from your own process, provided you have Accessibility permission.

### Key Framework Imports

```swift
import ApplicationServices  // Core AX API: AXUIElement, AXUIElementCopyAttributeValue, etc.
import AppKit               // NSWorkspace, NSRunningApplication
```

### The Core Type: `AXUIElement`

Every UI element in every macOS application is represented as an `AXUIElement` — an opaque reference (like a pointer) to a remote UI node in another process.

```swift
// Create a reference to an application's root element
let appElement = AXUIElementCreateApplication(pid)

// Create a reference to the entire system
let systemElement = AXUIElementCreateSystemWide()
```

> **Mental Model**: Think of `AXUIElement` as a remote reference (like an IPC handle). It doesn't *contain* the UI element — it *points to* it across process boundaries. Every query (`CopyAttributeValue`) is an IPC call to the target app.

---

## 2. The AXUIElement Model

### How Elements Are Organized

```
                    AXApplication (root)
                    ├── AXWindow
                    │   ├── AXToolbar
                    │   │   ├── AXButton "Close"
                    │   │   ├── AXButton "Minimize"  
                    │   │   └── AXButton "Zoom"
                    │   ├── AXScrollArea
                    │   │   └── AXWebArea
                    │   │       ├── AXGroup
                    │   │       │   ├── AXStaticText "Hello"
                    │   │       │   └── AXImage
                    │   │       └── AXList
                    │   │           ├── AXGroup (row 1)
                    │   │           └── AXGroup (row 2)
                    │   └── AXGroup (status bar)
                    └── AXMenuBar
                        ├── AXMenuBarItem "File"
                        ├── AXMenuBarItem "Edit"
                        └── AXMenuBarItem "View"
```

### Key Relationships

| Relationship | API | Description |
|---|---|---|
| **Parent** | `kAXParentAttribute` | The element that contains this one |
| **Children** | `kAXChildrenAttribute` | Ordered list of child elements |
| **Windows** | `kAXWindowsAttribute` | All windows owned by the app (from root) |
| **Main Window** | `kAXMainWindowAttribute` | The currently active window |
| **Focused Element** | `kAXFocusedUIElementAttribute` | Element with keyboard focus |

---

## 3. Permission Model

### Why Permission Is Needed

The Accessibility API can read *any* visible (and some invisible) UI element in *any* app. This is a massive privacy/security surface, so macOS gates it behind an explicit user permission.

### Checking Permission

```swift
import ApplicationServices

/// Check if this process is trusted to use Accessibility API
/// - Parameter prompt: If `true`, macOS may show the system prompt dialog
func isProcessTrusted(prompt: Bool) -> Bool {
    let options = [
        kAXTrustedCheckOptionPrompt.takeUnretainedValue() as String: prompt
    ] as CFDictionary
    return AXIsProcessTrustedWithOptions(options)
}
```

### Important Nuances

1. **Development vs Production**: During `cargo tauri dev`, the *Terminal* or *IDE* process needs permission (since it's the parent process). The bundled `.app` gets its own permission entry.

2. **Permission is per-executable**: If you rebuild and the binary path changes, you may need to re-grant permission.

3. **`kAXTrustedCheckOptionPrompt`**: When `true`, macOS shows a dialog pointing the user to System Settings. It only shows this *once* per session — subsequent calls with `prompt: true` silently return the current state.

4. **Info.plist requirement**: You must include `NSAccessibilityUsageDescription` in your `Info.plist`:
   ```xml
   <key>NSAccessibilityUsageDescription</key>
   <string>claw-kernel reads the frontmost app's UI structure.</string>
   ```

---

## 4. Roles — What Types of Elements Exist?

Every `AXUIElement` has a **role** (`kAXRoleAttribute`) that describes its semantic type. Here is a comprehensive reference:

### Window & Container Roles

| Role | Description | Typical Attributes | Example |
|------|-------------|-------------------|---------|
| `AXApplication` | Root element for an app | `AXWindows`, `AXMenuBar`, `AXFocusedUIElement` | The app itself |
| `AXWindow` | A window | `AXTitle`, `AXFrame`, `AXMinimized`, `AXFullScreen` | Main window |
| `AXSheet` | Modal sheet | `AXFrame` | Save dialog |
| `AXDrawer` | Drawer panel | `AXFrame` | Inspector drawer |
| `AXDialog` | Dialog window | `AXTitle`, `AXFrame` | Alert dialog |
| `AXGroup` | Generic container | `AXChildren` | `<div>` equivalent |
| `AXScrollArea` | Scrollable region | `AXHorizontalScrollBar`, `AXVerticalScrollBar` | Scroll view |
| `AXSplitGroup` | Split view container | `AXSplitters` | Split pane |
| `AXTabGroup` | Tab container | `AXTabs`, `AXValue` (selected tab) | Tab view |
| `AXToolbar` | Toolbar | `AXChildren` | App toolbar |

### Interactive Roles

| Role | Description | Key Attributes | Example |
|------|-------------|---------------|---------|
| `AXButton` | Clickable button | `AXTitle`, `AXEnabled` | "Save" button |
| `AXCheckBox` | Toggle checkbox | `AXValue` (0/1), `AXTitle` | Settings toggle |
| `AXRadioButton` | Radio option | `AXValue` (0/1), `AXTitle` | Option in a group |
| `AXRadioGroup` | Radio button group | `AXChildren` (radio buttons) | Mutually exclusive options |
| `AXSlider` | Value slider | `AXValue`, `AXMinValue`, `AXMaxValue` | Volume slider |
| `AXStepper` | Increment/decrement | `AXValue` | Number stepper |
| `AXPopUpButton` | Dropdown menu | `AXValue` (selected item) | Dropdown selector |
| `AXMenuButton` | Menu trigger | `AXTitle` | Toolbar menu button |
| `AXDisclosureTriangle` | Expand/collapse | `AXValue` (0/1) | Tree disclosure |
| `AXLink` | Hyperlink | `AXTitle`, `AXURL` | Web link |
| `AXColorWell` | Color picker | `AXValue` | Color selection |

### Text Roles

| Role | Description | Key Attributes | Example |
|------|-------------|---------------|---------|
| `AXStaticText` | Read-only text | `AXValue` (the text content) | Labels, headings |
| `AXTextField` | Editable text field | `AXValue`, `AXFocused`, `AXPlaceholderValue` | Input field |
| `AXTextArea` | Multi-line text | `AXValue`, `AXNumberOfCharacters` | Text editor |
| `AXHeading` | Heading element | `AXValue`, `AXHeadingLevel` | `<h1>`-`<h6>` equivalent |

### Table & List Roles

| Role | Description | Key Attributes | Example |
|------|-------------|---------------|---------|
| `AXTable` | Data table | `AXRows`, `AXColumns`, `AXHeader` | Table view |
| `AXRow` | Table/outline row | `AXIndex`, `AXSelected`, `AXDisclosureLevel` | Table row |
| `AXColumn` | Table column | `AXHeader`, `AXIndex` | Column definition |
| `AXCell` | Table cell | `AXValue`, `AXRow`, `AXColumn` | Cell content |
| `AXList` | Ordered list | `AXChildren`, `AXOrientation` | List view |
| `AXOutline` | Hierarchical list | `AXRows`, `AXColumns` | Outline/tree view |

### Menu Roles

| Role | Description | Key Attributes | Example |
|------|-------------|---------------|---------|
| `AXMenuBar` | App menu bar | `AXChildren` (menu bar items) | File, Edit, View... |
| `AXMenuBarItem` | Top-level menu | `AXTitle` | "File" |
| `AXMenu` | Dropdown menu | `AXChildren` (menu items) | File menu dropdown |
| `AXMenuItem` | Menu entry | `AXTitle`, `AXEnabled`, `AXMenuItemCmdChar` | "Save" |

### Web Content Roles (WebKit/Safari)

| Role | Description | Key Attributes | Example |
|------|-------------|---------------|---------|
| `AXWebArea` | Web page root | `AXChildren`, `AXURL` | Web page |
| `AXImage` | Image element | `AXDescription` (alt text), `AXURL` | `<img>` |
| `AXBanner` | Banner landmark | `AXChildren` | `<header>` |
| `AXNavigation` | Nav landmark | `AXChildren` | `<nav>` |
| `AXMain` | Main content | `AXChildren` | `<main>` |
| `AXContentInfo` | Content info | `AXChildren` | `<footer>` |

### Subrole — More Specific Classification

Some elements have a **subrole** (`kAXSubroleAttribute`) that refines the role:

```swift
// Example subroles
"AXCloseButton"      // The red close button (subrole of AXButton)
"AXMinimizeButton"   // The yellow minimize button
"AXZoomButton"       // The green zoom button
"AXToolbarButton"    // Button in a toolbar
"AXSecureTextField"  // Password field (subrole of AXTextField)
"AXSearchField"      // Search field
"AXSortButton"       // Column sort button
```

---

## 5. Attributes — What Data Can You Read?

### Reading an Attribute

```swift
/// Read a string attribute from an AXUIElement
func axString(_ element: AXUIElement, _ attribute: String) -> String? {
    var ref: CFTypeRef?
    let status = AXUIElementCopyAttributeValue(element, attribute as CFString, &ref)
    guard status == .success, let ref = ref else { return nil }
    guard CFGetTypeID(ref) == CFStringGetTypeID() else { return nil }
    return ref as? String
}
```

### Common Attribute Types

| Return Type | How to Read | Example Attributes |
|------------|-------------|-------------------|
| `String` | Cast to `CFString` → `String` | `kAXRoleAttribute`, `kAXTitleAttribute`, `kAXValueAttribute` |
| `Bool` | Cast to `CFBoolean` → `Bool` | `kAXEnabledAttribute`, `kAXFocusedAttribute`, `kAXSelectedAttribute` |
| `Int` / `Float` | Cast to `CFNumber` → `Int`/`Float` | `kAXNumberOfCharactersAttribute`, `kAXInsertionPointLineNumberAttribute` |
| `CGPoint` | `AXValue` with type `.cgPoint` | `kAXPositionAttribute` |
| `CGSize` | `AXValue` with type `.cgSize` | `kAXSizeAttribute` |
| `CGRect` | `AXValue` with type `.cgRect` | `"AXFrame"` (derived) |
| `[AXUIElement]` | Cast to `CFArray` → `[AXUIElement]` | `kAXChildrenAttribute`, `kAXWindowsAttribute` |
| `AXUIElement` | Direct cast | `kAXParentAttribute`, `kAXMainWindowAttribute` |
| `NSURL` / `String` | Cast to `CFURL` or `CFString` | `"AXURL"` |

### Reading `AXFrame` (Position + Size)

```swift
func axFrame(_ element: AXUIElement) -> CGRect? {
    var ref: CFTypeRef?
    guard AXUIElementCopyAttributeValue(element, "AXFrame" as CFString, &ref) == .success,
          let val = ref,
          CFGetTypeID(val) == AXValueGetTypeID()
    else { return nil }
    
    var rect = CGRect.zero
    guard AXValueGetValue(val as! AXValue, .cgRect, &rect) else { return nil }
    return rect
}
```

> **Coordinate System**: AX frames use **screen coordinates** with the origin at the **top-left** of the primary display. Y increases downward. This differs from AppKit's bottom-left origin but matches what you need for overlay drawing.

### Listing All Available Attributes

You can dynamically discover what attributes an element supports:

```swift
func axAttributeNames(_ element: AXUIElement) -> [String] {
    var names: CFArray?
    guard AXUIElementCopyAttributeNames(element, &names) == .success,
          let names = names as? [String]
    else { return [] }
    return names
}
```

This is extremely useful for inspection — different apps expose different custom attributes.

---

## 6. Actions — What Can You Trigger?

### Listing Actions

```swift
func axActions(_ element: AXUIElement) -> [String] {
    var names: CFArray?
    guard AXUIElementCopyActionNames(element, &names) == .success,
          let names = names as? [String]
    else { return [] }
    return names
}
```

### Common Actions

| Action | Description | Typical Elements |
|--------|-------------|-----------------|
| `kAXPressAction` ("AXPress") | Click/activate | Buttons, checkboxes, links |
| `kAXIncrementAction` ("AXIncrement") | Increase value | Sliders, steppers |
| `kAXDecrementAction` ("AXDecrement") | Decrease value | Sliders, steppers |
| `kAXConfirmAction` ("AXConfirm") | Confirm selection | Combo boxes |
| `kAXCancelAction` ("AXCancel") | Cancel operation | Sheets, dialogs |
| `kAXShowMenuAction` ("AXShowMenu") | Show context menu | Menu buttons |
| `kAXRaiseAction` ("AXRaise") | Bring to front | Windows |
| `kAXPickAction` ("AXPick") | Pick/select | Menu items |
| `kAXScrollToVisibleAction` ("AXScrollToVisible") | Scroll element into view | Any element |

### Performing an Action

```swift
func performAction(_ element: AXUIElement, action: String) -> Bool {
    let status = AXUIElementPerformAction(element, action as CFString)
    return status == .success
}
```

> **Note**: For the inspector, we primarily *read* actions to display them. We don't perform them — that would modify the target app's state.

---

## 7. The Element Hierarchy

### How to Think About the Tree

The AX tree mirrors the **view hierarchy** of the target app, but with important differences:

1. **Not 1:1 with views**: Some views are collapsed (container views without semantic meaning are often omitted). Some elements are synthesized (accessibility-specific elements not backed by a real view).

2. **App-dependent quality**: Well-built apps (Apple's own, major apps) have rich, deep trees. Poorly built apps may expose flat or incomplete trees.

3. **Dynamic**: The tree changes as the user interacts with the app. Elements are created/destroyed, attributes change, windows open/close.

### Expected Tree Structure by App Type

```
┌─────────────────┬────────────────────────────────────────────────────┐
│ App Type        │ Typical Tree Structure                             │
├─────────────────┼────────────────────────────────────────────────────┤
│ Native Cocoa    │ Deep, well-labeled. AXWindow → AXSplitGroup →     │
│ (Xcode, Finder) │ AXScrollArea → AXOutline → AXRow → AXCell         │
├─────────────────┼────────────────────────────────────────────────────┤
│ Electron Apps   │ AXWindow → AXWebArea → lots of AXGroup nesting.   │
│ (VS Code, Slack)│ Often flat; web ARIA roles map to AX roles.        │
├─────────────────┼────────────────────────────────────────────────────┤
│ SwiftUI Apps    │ Good structure but roles can be generic (AXGroup). │
│                 │ Labels come from `.accessibilityLabel()`.          │
├─────────────────┼────────────────────────────────────────────────────┤
│ Java/JVM Apps   │ Variable quality. Java Access Bridge may be needed │
│ (IntelliJ)     │ on some platforms (not macOS).                      │
├─────────────────┼────────────────────────────────────────────────────┤
│ Games / OpenGL  │ Usually no tree at all. AXWindow with no children. │
└─────────────────┴────────────────────────────────────────────────────┘
```

---

## 8. Getting the Frontmost Application

### Method 1: NSWorkspace (Recommended)

```swift
import AppKit

if let app = NSWorkspace.shared.frontmostApplication {
    let pid = app.processIdentifier
    let name = app.localizedName ?? "Unknown"
    let bundle = app.bundleIdentifier ?? "no.bundle"
    
    // Create the AX root element for this app
    let axApp = AXUIElementCreateApplication(pid)
    
    // Now you can traverse axApp's children
}
```

### Method 2: NSWorkspace Notifications (For Live Monitoring)

```swift
let center = NSWorkspace.shared.notificationCenter

let observer = center.addObserver(
    forName: NSWorkspace.didActivateApplicationNotification,
    object: nil,
    queue: .main
) { notification in
    guard let app = notification.userInfo?[NSWorkspace.applicationUserInfoKey] 
            as? NSRunningApplication else { return }
    
    // Filter to regular GUI apps (not background agents)
    guard app.activationPolicy == .regular else { return }
    
    let axApp = AXUIElementCreateApplication(app.processIdentifier)
    // Traverse and serialize the tree...
}
```

### Why `activationPolicy == .regular`?

macOS has three types of apps:
- **`.regular`**: Normal GUI apps with a Dock icon and menu bar (what we want)
- **`.accessory`**: Menu bar extras, background apps with occasional UI
- **`.prohibited`**: Pure background daemons, agents

We filter to `.regular` to avoid inspecting system daemons and menu bar utilities.

---

## 9. Traversing Children

### Basic Recursive Traversal

```swift
func traverseTree(_ element: AXUIElement, depth: Int = 0) {
    let role = axString(element, kAXRoleAttribute as String) ?? "Unknown"
    let title = axString(element, kAXTitleAttribute as String) ?? ""
    
    let indent = String(repeating: "  ", count: depth)
    print("\(indent)\(role): \(title)")
    
    // Get children
    var ref: CFTypeRef?
    guard AXUIElementCopyAttributeValue(
        element, 
        kAXChildrenAttribute as CFString, 
        &ref
    ) == .success,
    let children = ref as? [AXUIElement] else { return }
    
    for child in children {
        traverseTree(child, depth: depth + 1)
    }
}
```

### Guarding Against Infinite Loops and Explosion

Real-world AX trees can have:

1. **Cycles**: Element A's child points back to A (rare but possible in buggy apps)
2. **Deep nesting**: Some web content in Electron apps can nest 200+ levels deep
3. **Massive breadth**: Tables with 10,000+ rows, each with multiple cells

**Our defense strategy** (already implemented in the current code):

```swift
private let maxDepth = 120        // Don't go deeper than 120 levels
private let maxNodes = 60_000     // Stop after 60K nodes

private func serializeSubtree(
    _ element: AXUIElement,
    depth: Int,
    visited: inout Set<UInt>,     // Cycle detection via pointer identity
    nodeCount: inout Int,
    into output: inout String
) {
    // Depth guard
    if depth > maxDepth || nodeCount >= maxNodes { return }
    
    // Cycle detection: use the CFTypeRef pointer as a unique key
    let key = UInt(bitPattern: Unmanaged.passUnretained(element as CFTypeRef).toOpaque())
    if visited.contains(key) { return }
    visited.insert(key)
    
    nodeCount += 1
    // ... serialize this node and recurse into children
}
```

> **Why pointer-based cycle detection?** `AXUIElement` doesn't conform to `Hashable` or `Equatable` in a useful way. Two different `AXUIElement` references can point to the same remote element but have different pointer addresses. However, within a single traversal, if we see the same pointer twice, it's definitely a cycle. This is a pragmatic approach — not perfect, but sufficient.

---

## 10. Common Pitfalls and Edge Cases

### Pitfall 1: AXError Return Codes

`AXUIElementCopyAttributeValue` can fail for many reasons:

```swift
// Common AXError values
.success                    // All good
.attributeUnsupported       // Element doesn't have this attribute
.noValue                    // Attribute exists but has no value right now
.invalidUIElement           // Element was destroyed (app changed)
.cannotComplete             // App is busy or hung
.notImplemented             // The app's AX implementation is incomplete
.apiDisabled                // Accessibility permission not granted
```

**Best Practice**: Always check the error. Don't assume an element or attribute exists.

### Pitfall 2: Element Lifetime

`AXUIElement` references can become **stale**. If the target app closes a window or removes a UI element, any references to it will return `invalidUIElement`. This is normal — handle it gracefully.

### Pitfall 3: Thread Safety

`AXUIElement` calls are **synchronous IPC** — they block until the target app responds. If the target app is hung or spinning:
- Your thread will block
- Timeout is roughly 5-6 seconds per call
- Consider running heavy traversals on a background thread

### Pitfall 4: Menu Bar and Hidden Windows

- The `AXMenuBar` is always present even when menus aren't open
- Hidden windows (minimized, behind other windows) are still in the tree
- Some apps have accessibility elements that aren't visible on screen

### Pitfall 5: Coordinate Systems

```
Screen coordinate space (AX uses this):
  ┌──────────────────────────┐
  │ (0,0)              (W,0) │  ← Primary display top-left is origin
  │                          │
  │         Display          │
  │                          │
  │ (0,H)              (W,H) │
  └──────────────────────────┘

AppKit coordinate space (NSView, NSWindow):
  ┌──────────────────────────┐
  │ (0,H)              (W,H) │
  │                          │
  │         Display          │
  │                          │
  │ (0,0)              (W,0) │  ← Bottom-left is origin
  └──────────────────────────┘
```

When drawing overlays with `NSPanel`/`NSWindow`, you'll need to convert:
```swift
let screenHeight = NSScreen.main?.frame.height ?? 0
let axFrame: CGRect = ...  // from AXUIElement
let appKitY = screenHeight - axFrame.origin.y - axFrame.height
```

---

## 11. Performance Considerations

### Query Cost

Each `AXUIElementCopyAttributeValue` call is an **IPC round-trip** to the target process:
- Typical latency: 0.01–0.5ms per attribute
- For a tree with 1,000 nodes × 10 attributes each = 10,000 IPC calls
- That's 10ms–5s depending on the target app's responsiveness

### Optimization Strategies

1. **Lazy loading**: Only fetch children when the user expands a node
2. **Attribute batching**: Fetch only essential attributes (role, title, frame) during traversal; fetch the rest on demand when a node is selected
3. **Background traversal**: Run on a background thread, stream results
4. **Caching with invalidation**: Cache the tree structure, invalidate when `AXUIElementNotification` fires

### What We'll Use for V1

For the first iteration, we'll do a **full eager traversal** (current approach) because:
- It's simpler to implement
- Most apps have < 5,000 nodes (handles fine)
- We need to understand the full picture before optimizing

We'll add lazy loading in a later iteration if performance becomes an issue with very large trees (e.g., inspecting VS Code).

---

## Summary

| Concept | Key Takeaway |
|---------|-------------|
| `AXUIElement` | Opaque remote reference to a UI element in another process |
| Permission | Required, per-executable, checked via `AXIsProcessTrustedWithOptions` |
| Role | Semantic type of an element (`AXButton`, `AXWindow`, etc.) |
| Attributes | Named properties: `kAXTitleAttribute`, `kAXFrameAttribute`, etc. |
| Children | Accessed via `kAXChildrenAttribute`, returns `[AXUIElement]` |
| Actions | Things you can trigger: `AXPress`, `AXIncrement`, etc. |
| Traversal | Recursive DFS with cycle detection and depth/node limits |
| Coordinates | Screen space, top-left origin (convert for AppKit drawing) |

> **Next**: [02-project-architecture.md](./02-project-architecture.md) — How to organize the Swift, Rust, and React code for scalability.
