# 03 — Swift Accessibility Layer (Step-by-Step)

> **Goal**: Refactor the existing Swift code into a modular architecture and add JSON serialization for React consumption.

---

## Table of Contents

1. [What We're Changing](#1-what-were-changing)
2. [Step 1: Create AXHelpers.swift](#step-1-create-axhelpersswift)
3. [Step 2: Create AXNodeModel.swift](#step-2-create-axnodemodelswift)
4. [Step 3: Create AXTraversal.swift](#step-3-create-axtraversalswift)
5. [Step 4: Create AXPermission.swift](#step-4-create-axpermissionswift)
6. [Step 5: Create JSONSerializer.swift](#step-5-create-jsonserializerswift)
7. [Step 6: Create XMLSerializer.swift](#step-6-create-xmlserializerswift)
8. [Step 7: Create FrontmostMonitor.swift](#step-7-create-frontmostmonitorswift)
9. [Step 8: Create Exports.swift](#step-8-create-exportsswift)
10. [Step 9: Update Package.swift](#step-9-update-packageswift)
11. [Understanding the Traversal in Detail](#understanding-the-traversal-in-detail)
12. [Testing the Swift Layer](#testing-the-swift-layer)

---

## 1. What We're Changing

### Current State: One File Does Everything

```
ClawAccessibility.swift (305 lines)
├── XML escape helpers (lines 9-28)
├── AX attribute readers (lines 42-91) 
├── Serialization to XML (lines 96-166)
├── Permission check (lines 171-179)
├── AX dump to file (lines 182-222)
└── Frontmost monitor (lines 226-305)
```

### Target State: Modular by Responsibility

```
Sources/ClawAccessibility/
├── Core/
│   ├── AXHelpers.swift          # Pure AX attribute reading (extracted from lines 42-91)
│   ├── AXTraversal.swift        # Tree walking (extracted + modified from lines 96-144)
│   └── AXPermission.swift       # Permission check (extracted from lines 171-179)
├── Models/
│   └── AXNodeModel.swift        # NEW: Codable data model
├── Serialization/
│   ├── JSONSerializer.swift     # NEW: Tree → JSON
│   └── XMLSerializer.swift      # Extracted from lines 9-38 + 96-166
├── Monitor/
│   └── FrontmostMonitor.swift   # Extracted from lines 226-305
└── FFI/
    └── Exports.swift            # @_cdecl functions, calls into other modules
```

> **Note on Swift Package Manager**: SPM treats all `.swift` files under `Sources/<TargetName>/` as part of the same module regardless of subdirectory structure. Subdirectories are purely for organization — no additional `Package.swift` configuration is needed for them.

---

## Step 1: Create `AXHelpers.swift`

**File**: `Sources/ClawAccessibility/Core/AXHelpers.swift`

This file contains **pure utility functions** for reading attributes from `AXUIElement`. No side effects, no state.

```swift
import ApplicationServices
import Foundation

// MARK: - AXUIElement Attribute Readers

/// Read a string attribute. Returns nil if the attribute doesn't exist or isn't a string.
func axString(_ element: AXUIElement, _ attr: String) -> String? {
    var ref: CFTypeRef?
    guard AXUIElementCopyAttributeValue(element, attr as CFString, &ref) == .success,
          let ref = ref
    else { return nil }
    // Ensure we're dealing with a CFString
    guard CFGetTypeID(ref) == CFStringGetTypeID() else { return nil }
    return ref as? String
}

/// Read a boolean attribute.
func axBool(_ element: AXUIElement, _ attr: String) -> Bool? {
    var ref: CFTypeRef?
    guard AXUIElementCopyAttributeValue(element, attr as CFString, &ref) == .success,
          let ref = ref
    else { return nil }
    guard CFGetTypeID(ref) == CFBooleanGetTypeID() else { return nil }
    return CFBooleanGetValue(ref as! CFBoolean)
}

/// Read an integer attribute.
func axInt(_ element: AXUIElement, _ attr: String) -> Int? {
    var ref: CFTypeRef?
    guard AXUIElementCopyAttributeValue(element, attr as CFString, &ref) == .success,
          let ref = ref
    else { return nil }
    guard CFGetTypeID(ref) == CFNumberGetTypeID() else { return nil }
    return ref as? Int
}

/// Read the frame (position + size) of an element.
/// Returns a CGRect in screen coordinates (origin at top-left of primary display).
func axFrame(_ element: AXUIElement) -> CGRect? {
    var ref: CFTypeRef?
    // "AXFrame" is a computed attribute combining position and size
    guard AXUIElementCopyAttributeValue(element, "AXFrame" as CFString, &ref) == .success,
          let val = ref,
          CFGetTypeID(val) == AXValueGetTypeID()
    else { return nil }
    
    var rect = CGRect.zero
    guard AXValueGetValue(val as! AXValue, .cgRect, &rect) else { return nil }
    return rect
}

/// Read all children of an element. Returns empty array if none.
func axChildren(_ element: AXUIElement) -> [AXUIElement] {
    var ref: CFTypeRef?
    guard AXUIElementCopyAttributeValue(
        element,
        kAXChildrenAttribute as CFString,
        &ref
    ) == .success,
    let arr = ref as? [AXUIElement]
    else { return [] }
    return arr
}

/// Get the list of action names this element supports.
func axActionNames(_ element: AXUIElement) -> [String] {
    var names: CFArray?
    guard AXUIElementCopyActionNames(element, &names) == .success,
          let names = names as? [String]
    else { return [] }
    return names
}

/// Get ALL attribute names this element supports (for dynamic inspection).
func axAttributeNames(_ element: AXUIElement) -> [String] {
    var names: CFArray?
    guard AXUIElementCopyAttributeNames(element, &names) == .success,
          let names = names as? [String]
    else { return [] }
    return names
}

/// Generate a unique key for cycle detection based on the CFTypeRef pointer.
///
/// Why pointer-based? AXUIElement doesn't conform to Hashable. Within a single
/// traversal, seeing the same pointer twice guarantees a cycle. Across traversals,
/// pointers may be reused — but that's fine because we create a fresh visited set
/// each time.
func axElementKey(_ element: AXUIElement) -> UInt {
    UInt(bitPattern: Unmanaged.passUnretained(element as CFTypeRef).toOpaque())
}
```

### What You're Learning Here

1. **`CFTypeRef` pattern**: All AX attributes come back as `CFTypeRef?` (basically `Any?`). You must check the type with `CFGetTypeID()` before casting.

2. **Error handling**: `AXUIElementCopyAttributeValue` returns an `AXError` enum. We pattern-match on `.success` — any other value means the attribute doesn't exist, the element is stale, or permission is denied.

3. **`AXValue` wrapping**: Geometric types (point, size, rect) are wrapped in `AXValue`, which requires `AXValueGetValue()` with a type hint to unwrap.

---

## Step 2: Create `AXNodeModel.swift`

**File**: `Sources/ClawAccessibility/Models/AXNodeModel.swift`

This is the **data model** — the contract between Swift and the rest of the system.

```swift
import Foundation

/// Represents a single node in the accessibility tree.
///
/// This is a **snapshot** — it captures the state at traversal time.
/// The original AXUIElement reference is NOT stored (it can't cross FFI).
///
/// Conforms to `Codable` for automatic JSON/XML encoding via Foundation.
struct AXNodeModel: Codable {
    /// Unique identifier for this node within a single traversal.
    /// Generated from the AXUIElement pointer hash + depth index.
    /// NOT stable across traversals (pointers are recycled).
    let id: String
    
    /// The accessibility role: "AXButton", "AXWindow", "AXStaticText", etc.
    /// Always present (defaults to "AXUnknown" if the element has no role).
    let role: String
    
    /// More specific role: "AXCloseButton", "AXSearchField", etc.
    /// Only present for elements with a subrole.
    let subrole: String?
    
    /// The title attribute — typically the visible label of buttons, windows, menu items.
    let title: String?
    
    /// The accessibility description — a more detailed label for screen readers.
    let description: String?
    
    /// The label attribute (less common; often overlaps with title or description).
    let label: String?
    
    /// Help text — tooltip or extended description.
    let help: String?
    
    /// The value attribute, stringified.
    /// For text fields: the text content.
    /// For checkboxes: "0" or "1".
    /// For sliders: the numeric value as string.
    let value: String?
    
    /// The element's frame in screen coordinates.
    let frame: FrameModel?
    
    /// Whether the element is enabled (clickable, editable).
    let enabled: Bool?
    
    /// Whether the element currently has keyboard focus.
    let focused: Bool?
    
    /// Whether the element is selected (in a list/table).
    let selected: Bool?
    
    /// List of actions this element supports: ["AXPress", "AXShowMenu"], etc.
    let actions: [String]
    
    /// All attribute names this element supports (for dynamic inspection).
    let attributes: [String]
    
    /// Child nodes. Empty array for leaf nodes.
    let children: [AXNodeModel]
    
    /// Number of direct children. Matches children.count but useful for
    /// lazy-loading scenarios where children array might be empty but
    /// we know children exist.
    let childCount: Int
}

/// Screen-coordinate frame of an element.
///
/// Origin is the top-left of the primary display.
/// Y increases downward (matching AX coordinate system).
struct FrameModel: Codable {
    let x: Double
    let y: Double
    let width: Double
    let height: Double
    
    init(from rect: CGRect) {
        self.x = Double(rect.origin.x)
        self.y = Double(rect.origin.y)
        self.width = Double(rect.size.width)
        self.height = Double(rect.size.height)
    }
}

/// Metadata about the inspected application.
struct AppInfoModel: Codable {
    let pid: Int32
    let bundleIdentifier: String?
    let name: String?
}

/// Top-level response from the Swift layer.
/// Contains app metadata + the root of the AX tree.
struct AXTreeResponse: Codable {
    let app: AppInfoModel
    let root: AXNodeModel
    let nodeCount: Int
    let truncated: Bool  // true if we hit maxDepth or maxNodes
}
```

### Why `Codable`?

Swift's `Codable` protocol auto-generates JSON encoding/decoding. Since all our fields are basic types (`String`, `Bool`, `Int`, arrays, optionals), `Codable` works out of the box:

```swift
let encoder = JSONEncoder()
let data = try encoder.encode(response)  // AXTreeResponse → Data (JSON bytes)
let json = String(data: data, encoding: .utf8)!  // → JSON string
```

No manual string building, no escaping bugs, no format mismatches.

### Why `id` as a String?

We generate IDs like `"n_0"`, `"n_1"`, `"n_2"` using a traversal counter. This gives us:
- **Uniqueness within a traversal**: Guaranteed by the counter
- **Cheap to generate**: No UUID overhead
- **Useful in React**: `key={node.id}` for efficient reconciliation
- **Not stable across traversals**: Intentional — the tree changes, IDs shouldn't imply persistence

---

## Step 3: Create `AXTraversal.swift`

**File**: `Sources/ClawAccessibility/Core/AXTraversal.swift`

This is the **core of the inspector** — the recursive tree walker that builds `AXNodeModel`.

```swift
import ApplicationServices
import Foundation

// MARK: - Traversal Configuration

/// Controls traversal behavior. Adjust these for different scenarios:
/// - Full inspection: default values
/// - Quick preview: lower maxDepth, fewer attributes
/// - Lazy loading: maxDepth = 1 for expand-on-demand
struct TraversalConfig {
    /// Maximum recursion depth. Prevents stack overflow on deeply nested trees.
    /// 120 handles even the most complex Electron apps.
    let maxDepth: Int
    
    /// Maximum total nodes to visit. Prevents runaway traversal on apps
    /// with massive tables (e.g., Finder with 100K files listed).
    let maxNodes: Int
    
    /// Default configuration for full tree extraction.
    static let `default` = TraversalConfig(maxDepth: 120, maxNodes: 60_000)
    
    /// Shallow config for quick previews or lazy-loading roots.
    static let shallow = TraversalConfig(maxDepth: 2, maxNodes: 500)
}

// MARK: - Traversal State

/// Mutable state carried through a single traversal.
/// Passed as `inout` to avoid allocation overhead.
private struct TraversalState {
    var visited: Set<UInt> = []   // Cycle detection
    var nodeCount: Int = 0        // Total nodes visited
    var nextId: Int = 0           // ID counter for unique node IDs
    var truncated: Bool = false   // Did we hit limits?
    
    mutating func nextNodeId() -> String {
        let id = "n_\(nextId)"
        nextId += 1
        return id
    }
}

// MARK: - Public API

/// Traverse the accessibility tree rooted at `element`.
///
/// This function performs a depth-first traversal, reading attributes from each
/// AXUIElement and building an AXNodeModel tree. It guards against:
/// - Cycles (via pointer-based visited set)
/// - Excessive depth (via maxDepth)
/// - Excessive breadth (via maxNodes)
///
/// - Parameters:
///   - element: The root AXUIElement (typically from AXUIElementCreateApplication)
///   - config: Traversal limits and options
/// - Returns: A tuple of the root model node, total node count, and whether traversal was truncated
func traverseAccessibilityTree(
    root element: AXUIElement,
    config: TraversalConfig = .default
) -> (root: AXNodeModel, nodeCount: Int, truncated: Bool) {
    var state = TraversalState()
    let rootNode = buildNode(element, depth: 0, config: config, state: &state)
    return (rootNode, state.nodeCount, state.truncated)
}

// MARK: - Private Recursive Builder

/// Recursively build an AXNodeModel from an AXUIElement.
///
/// Each call:
/// 1. Checks depth/node limits and cycle detection
/// 2. Reads all relevant attributes from the AXUIElement (IPC calls)
/// 3. Recursively processes children
/// 4. Returns a fully populated AXNodeModel
private func buildNode(
    _ element: AXUIElement,
    depth: Int,
    config: TraversalConfig,
    state: inout TraversalState
) -> AXNodeModel {
    // --- Guard: depth and node limits ---
    if depth > config.maxDepth || state.nodeCount >= config.maxNodes {
        state.truncated = true
        return AXNodeModel(
            id: state.nextNodeId(),
            role: "AXTruncated",
            subrole: nil,
            title: "Tree truncated (depth: \(depth))",
            description: nil, label: nil, help: nil, value: nil,
            frame: nil, enabled: nil, focused: nil, selected: nil,
            actions: [], attributes: [], children: [], childCount: 0
        )
    }
    
    // --- Guard: cycle detection ---
    let key = axElementKey(element)
    if state.visited.contains(key) {
        return AXNodeModel(
            id: state.nextNodeId(),
            role: "AXCycleRef",
            subrole: nil,
            title: "Cycle detected",
            description: nil, label: nil, help: nil, value: nil,
            frame: nil, enabled: nil, focused: nil, selected: nil,
            actions: [], attributes: [], children: [], childCount: 0
        )
    }
    state.visited.insert(key)
    state.nodeCount += 1
    
    // --- Read attributes ---
    // Every line here is an IPC call to the target process.
    // We read the "essential" attributes that are useful for inspection.
    let role = axString(element, kAXRoleAttribute as String) ?? "AXUnknown"
    let subrole = axString(element, kAXSubroleAttribute as String)
    let title = axString(element, kAXTitleAttribute as String)
    let description = axString(element, kAXDescriptionAttribute as String)
    let label = axString(element, kAXLabelValueAttribute as String)
    let help = axString(element, kAXHelpAttribute as String)
    let value = axString(element, kAXValueAttribute as String)
    let frame = axFrame(element).map { FrameModel(from: $0) }
    let enabled = axBool(element, kAXEnabledAttribute as String)
    let focused = axBool(element, kAXFocusedAttribute as String)
    let selected = axBool(element, kAXSelectedAttribute as String)
    let actions = axActionNames(element)
    let attributes = axAttributeNames(element)
    
    // --- Recurse into children ---
    let axKids = axChildren(element)
    let children: [AXNodeModel] = axKids.map { child in
        buildNode(child, depth: depth + 1, config: config, state: &state)
    }
    
    return AXNodeModel(
        id: state.nextNodeId(),
        role: role,
        subrole: subrole,
        title: title,
        description: description,
        label: label,
        help: help,
        value: value,
        frame: frame,
        enabled: enabled,
        focused: focused,
        selected: selected,
        actions: actions,
        attributes: attributes,
        children: children,
        childCount: axKids.count
    )
}
```

### Understanding the Traversal in Detail

#### Why Depth-First Search (DFS)?

We use DFS (via recursion) rather than BFS because:
- The AX tree is narrow and deep (typical: 5-20 children per node, 10-30 depth)
- DFS matches the natural reading order (parent before children)
- Stack memory usage is proportional to depth (~120 stack frames max), not breadth
- Results in proper nesting in the output model

#### The IPC Cost of Each Node

For each node, we make approximately **13 IPC calls** to the target process:
```
role, subrole, title, description, label, help, value,
frame, enabled, focused, selected, actions, attributeNames
```

For a tree with 1,000 nodes: ~13,000 IPC calls. At ~0.05ms each = ~650ms total traversal time. This is acceptable for V1 but worth noting for future optimization.

#### What Is `kAXLabelValueAttribute`?

This is a less-common attribute. Some apps (especially those using newer accessibility APIs) provide a label value separate from title/description:
- `kAXTitleAttribute` → Window title, button text
- `kAXDescriptionAttribute` → VoiceOver description
- `kAXLabelValueAttribute` → Explicit label (newer API)

We read all three to capture maximum information.

---

## Step 4: Create `AXPermission.swift`

**File**: `Sources/ClawAccessibility/Core/AXPermission.swift`

```swift
import ApplicationServices

/// Check whether this process is trusted for Accessibility API access.
///
/// - Parameter prompt: If `true`, macOS may show the system dialog directing
///   the user to System Settings → Privacy & Security → Accessibility.
///   The dialog only appears once per user session.
///
/// - Returns: `true` if the app has Accessibility permission.
///
/// ## How macOS Permission Works
///
/// 1. First call with `prompt: true`: System shows a dialog with "Open System Settings"
/// 2. User enables the app in System Settings
/// 3. Subsequent calls with `prompt: false` return `true`
///
/// During development (`cargo tauri dev`), the Terminal or IDE process
/// needs permission instead of the app itself.
func isAccessibilityTrusted(prompt: Bool) -> Bool {
    let opts = [
        kAXTrustedCheckOptionPrompt.takeUnretainedValue() as String: prompt
    ] as CFDictionary
    return AXIsProcessTrustedWithOptions(opts)
}
```

---

## Step 5: Create `JSONSerializer.swift`

**File**: `Sources/ClawAccessibility/Serialization/JSONSerializer.swift`

```swift
import Foundation

/// Serialize an AXTreeResponse to a JSON string.
///
/// Uses Foundation's JSONEncoder with:
/// - No pretty printing (smaller payload for IPC)
/// - Sorted keys (deterministic output, useful for diffing/debugging)
///
/// ## Why Foundation Codable?
///
/// 1. Zero manual string building = zero escaping bugs
/// 2. Type-safe: compiler ensures all fields are encoded
/// 3. Adding a new field to AXNodeModel automatically includes it in JSON
/// 4. Matches Rust's serde_json deserialization exactly
enum JSONSerializer {
    
    /// Serialize the full tree response to JSON.
    ///
    /// Returns the JSON string, or an error message prefixed with "error:".
    static func serialize(_ response: AXTreeResponse) -> String {
        let encoder = JSONEncoder()
        // Use sorted keys for deterministic output
        encoder.outputFormatting = [.sortedKeys]
        
        do {
            let data = try encoder.encode(response)
            return String(data: data, encoding: .utf8) ?? "error:UTF-8 encoding failed"
        } catch {
            return "error:JSON encoding failed: \(error.localizedDescription)"
        }
    }
    
    /// Serialize just a single node (useful for partial updates in the future).
    static func serializeNode(_ node: AXNodeModel) -> String {
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.sortedKeys]
        
        do {
            let data = try encoder.encode(node)
            return String(data: data, encoding: .utf8) ?? "error:UTF-8 encoding failed"
        } catch {
            return "error:JSON encoding failed: \(error.localizedDescription)"
        }
    }
}
```

### Why an `enum` Instead of a `struct`/`class`?

We use an `enum` with no cases (caseless enum) as a **namespace** for static methods. This is a Swift convention for utility types that shouldn't be instantiated:

```swift
enum JSONSerializer {  // Cannot be instantiated (no cases)
    static func serialize(...) -> String { ... }
}

// Compare with alternatives:
// struct JSONSerializer { ... }  // Can be accidentally instantiated
// class JSONSerializer { ... }   // Same problem + reference type overhead
```

---

## Step 6: Create `XMLSerializer.swift`

**File**: `Sources/ClawAccessibility/Serialization/XMLSerializer.swift`

This is the refactored version of the existing XML logic, now working on `AXNodeModel` instead of `AXUIElement`.

```swift
import Foundation

/// Serialize an AXTreeResponse to XML string.
///
/// This is kept for backward compatibility and file export.
/// The primary IPC format is JSON (see JSONSerializer).
enum XMLSerializer {
    
    static func serialize(_ response: AXTreeResponse) -> String {
        var xml = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"
        xml += "<accessibility"
        xml += " pid=\"\(response.app.pid)\""
        if let bid = response.app.bundleIdentifier {
            xml += attr("bundleIdentifier", bid)
        }
        if let name = response.app.name {
            xml += attr("name", name)
        }
        xml += " nodes=\"\(response.nodeCount)\""
        xml += ">\n"
        serializeNode(response.root, into: &xml)
        xml += "\n</accessibility>\n"
        return xml
    }
    
    // MARK: - Private
    
    private static func serializeNode(_ node: AXNodeModel, into xml: inout String) {
        xml += "<node"
        xml += attr("role", node.role)
        xml += optAttr("subrole", node.subrole)
        xml += optAttr("title", node.title)
        xml += optAttr("label", node.label)
        xml += optAttr("description", node.description)
        xml += optAttr("help", node.help)
        xml += optAttr("value", node.value)
        
        if let f = node.frame {
            let frameStr = String(
                format: "{\"x\":%.2f,\"y\":%.2f,\"width\":%.2f,\"height\":%.2f}",
                f.x, f.y, f.width, f.height
            )
            xml += attr("frame", frameStr)
        }
        
        if let e = node.enabled { xml += " enabled=\"\(e)\"" }
        if let f = node.focused { xml += " focused=\"\(f)\"" }
        if let s = node.selected { xml += " selected=\"\(s)\"" }
        
        if !node.actions.isEmpty {
            xml += attr("actions", node.actions.joined(separator: ","))
        }
        
        if node.children.isEmpty {
            xml += "/>"
        } else {
            xml += ">"
            for child in node.children {
                serializeNode(child, into: &xml)
            }
            xml += "</node>"
        }
    }
    
    private static func xmlEscape(_ s: String) -> String {
        var out = ""
        out.reserveCapacity(s.count)
        for ch in s.unicodeScalars {
            switch ch {
            case "&": out += "&amp;"
            case "<": out += "&lt;"
            case ">": out += "&gt;"
            case "\"": out += "&quot;"
            case "'": out += "&apos;"
            default:
                if ch.value < 0x20 && ch != "\n" && ch != "\r" && ch != "\t" {
                    out += "&#\(ch.value);"
                } else {
                    out.unicodeScalars.append(ch)
                }
            }
        }
        return out
    }
    
    private static func attr(_ name: String, _ value: String) -> String {
        " \(name)=\"\(xmlEscape(value))\""
    }
    
    private static func optAttr(_ name: String, _ value: String?) -> String {
        guard let v = value, !v.isEmpty else { return "" }
        return attr(name, v)
    }
}
```

---

## Step 7: Create `FrontmostMonitor.swift`

**File**: `Sources/ClawAccessibility/Monitor/FrontmostMonitor.swift`

```swift
import AppKit
import Foundation

/// Monitors the frontmost application and triggers callbacks when it changes.
///
/// ## How It Works
///
/// Registers observers on `NSWorkspace.shared.notificationCenter` for:
/// - `didActivateApplicationNotification`: User clicked on or Cmd+Tab to another app
/// - `didLaunchApplicationNotification`: A new app was launched
///
/// When either fires, we:
/// 1. Check if it's a regular GUI app (not a background daemon)
/// 2. If Accessibility is trusted, re-traverse the AX tree and save to file
/// 3. Call the Rust callback with the bundle identifier
///
/// ## Thread Safety
///
/// All notifications are delivered on `.main` queue. The callback to Rust
/// is also called on the main thread. This is safe because:
/// - swift-rs callbacks are designed for main-thread use
/// - NSWorkspace notifications are always main-thread
///
/// ## Why Filter to `.regular`?
///
/// macOS activationPolicy:
/// - `.regular`: Normal apps (Finder, Safari, VS Code) — what we want
/// - `.accessory`: Menu bar utilities, background helpers — not useful to inspect
/// - `.prohibited`: Pure daemons — no UI at all
enum FrontmostMonitor {
    
    typealias FrontmostCallback = @convention(c) (UnsafePointer<CChar>?) -> Void
    
    private static var observers: [NSObjectProtocol] = []
    
    /// Start monitoring frontmost app changes.
    ///
    /// - Parameters:
    ///   - callbackPtr: C function pointer from Rust to call with bundle ID
    ///   - dumpPath: File path to write the AX XML dump to (can be empty to skip)
    static func start(callbackPtr: UnsafeRawPointer, dumpPath: String) {
        let callback = unsafeBitCast(callbackPtr, to: FrontmostCallback.self)
        
        // Remove any existing observers first
        stop()
        
        let center = NSWorkspace.shared.notificationCenter
        
        let obsActivate = center.addObserver(
            forName: NSWorkspace.didActivateApplicationNotification,
            object: nil,
            queue: .main
        ) { note in
            guard let app = note.userInfo?[NSWorkspace.applicationUserInfoKey]
                    as? NSRunningApplication else { return }
            handleAppChange(app, callback: callback, dumpPath: dumpPath)
        }
        
        let obsLaunch = center.addObserver(
            forName: NSWorkspace.didLaunchApplicationNotification,
            object: nil,
            queue: .main
        ) { note in
            guard let app = note.userInfo?[NSWorkspace.applicationUserInfoKey]
                    as? NSRunningApplication else { return }
            // Don't monitor our own launch
            guard app.processIdentifier != ProcessInfo.processInfo.processIdentifier else { return }
            handleAppChange(app, callback: callback, dumpPath: dumpPath)
        }
        
        observers = [obsActivate, obsLaunch]
        
        // Immediately process the current frontmost app
        if let app = NSWorkspace.shared.frontmostApplication {
            handleAppChange(app, callback: callback, dumpPath: dumpPath)
        }
    }
    
    /// Stop monitoring. Removes all NSWorkspace observers.
    static func stop() {
        let center = NSWorkspace.shared.notificationCenter
        for observer in observers {
            center.removeObserver(observer)
        }
        observers.removeAll()
    }
    
    // MARK: - Private
    
    private static func handleAppChange(
        _ app: NSRunningApplication,
        callback: FrontmostCallback,
        dumpPath: String
    ) {
        // Filter to regular GUI apps
        guard app.activationPolicy == .regular else { return }
        
        // Optionally dump AX tree to file
        if isAccessibilityTrusted(prompt: false), !dumpPath.isEmpty {
            dumpToFile(app: app, path: dumpPath)
        }
        
        // Notify Rust with the bundle identifier
        let payload: String
        if let bid = app.bundleIdentifier, !bid.isEmpty {
            payload = bid
        } else {
            payload = "pid:\(app.processIdentifier)"
        }
        
        payload.withCString { ptr in
            guard let dup = strdup(ptr) else { return }
            callback(UnsafePointer(dup))
            free(dup)
        }
    }
    
    /// Write the AX tree for the given app to a file (XML format).
    private static func dumpToFile(app: NSRunningApplication, path: String) {
        let pid = app.processIdentifier
        let axApp = AXUIElementCreateApplication(pid)
        let (root, nodeCount, truncated) = traverseAccessibilityTree(root: axApp)
        
        let response = AXTreeResponse(
            app: AppInfoModel(
                pid: pid,
                bundleIdentifier: app.bundleIdentifier,
                name: app.localizedName
            ),
            root: root,
            nodeCount: nodeCount,
            truncated: truncated
        )
        
        let xml = XMLSerializer.serialize(response)
        let url = URL(fileURLWithPath: path)
        
        do {
            try xml.write(to: url, atomically: true, encoding: .utf8)
            let label = app.bundleIdentifier ?? "pid:\(pid)"
            print("[ClawAccessibility] write OK path=\(path) bytes=\(xml.utf8.count) app=\(label)")
        } catch {
            print("[ClawAccessibility] write FAILED path=\(path) error=\(error.localizedDescription)")
        }
    }
}
```

---

## Step 8: Create `Exports.swift`

**File**: `Sources/ClawAccessibility/FFI/Exports.swift`

This is the **only file Rust talks to**. Every `@_cdecl` function lives here.

```swift
import AppKit
import ApplicationServices
import Foundation
import SwiftRs

// MARK: - FFI Exports (Rust ↔ Swift boundary)
//
// Naming convention: claw_ax_<verb>_<noun>
// All functions here are called from Rust via swift-rs.
// They must use C-compatible types (Bool, SRString, UnsafeRawPointer).

/// Check if this process has Accessibility permission.
///
/// Called from Rust: `swift!(fn claw_ax_is_process_trusted(prompt: Bool) -> Bool)`
@_cdecl("claw_ax_is_process_trusted")
public func claw_ax_is_process_trusted(prompt: Bool) -> Bool {
    isAccessibilityTrusted(prompt: prompt)
}

/// Get the accessibility tree of the frontmost app as a JSON string.
///
/// Returns a JSON string in one of two forms:
/// - Success: `{"app": {...}, "root": {...}, "nodeCount": N, "truncated": false}`
/// - Error: `"error:description"`
///
/// Called from Rust: `swift!(fn claw_ax_get_tree_json() -> SRString)`
@_cdecl("claw_ax_get_tree_json")
public func claw_ax_get_tree_json() -> SRString {
    guard isAccessibilityTrusted(prompt: false) else {
        return SRString("error:Accessibility not trusted")
    }
    
    guard let app = NSWorkspace.shared.frontmostApplication else {
        return SRString("error:No frontmost application")
    }
    
    let pid = app.processIdentifier
    let axApp = AXUIElementCreateApplication(pid)
    
    let (root, nodeCount, truncated) = traverseAccessibilityTree(root: axApp)
    
    let response = AXTreeResponse(
        app: AppInfoModel(
            pid: pid,
            bundleIdentifier: app.bundleIdentifier,
            name: app.localizedName
        ),
        root: root,
        nodeCount: nodeCount,
        truncated: truncated
    )
    
    let json = JSONSerializer.serialize(response)
    return SRString(json)
}

/// Dump the frontmost app's AX tree to an XML file.
///
/// Called from Rust: `swift!(fn claw_ax_dump_frontmost_to_file(path: &SRString) -> SRString)`
@_cdecl("claw_ax_dump_frontmost_to_file")
public func claw_ax_dump_frontmost_to_file(path: SRString) -> SRString {
    guard let app = NSWorkspace.shared.frontmostApplication else {
        return SRString("error:No frontmost application.")
    }
    
    guard isAccessibilityTrusted(prompt: false) else {
        return SRString("error:Accessibility not trusted")
    }
    
    let pid = app.processIdentifier
    let axApp = AXUIElementCreateApplication(pid)
    let pathStr = path.toString()
    
    let (root, nodeCount, truncated) = traverseAccessibilityTree(root: axApp)
    
    let response = AXTreeResponse(
        app: AppInfoModel(
            pid: pid,
            bundleIdentifier: app.bundleIdentifier,
            name: app.localizedName
        ),
        root: root,
        nodeCount: nodeCount,
        truncated: truncated
    )
    
    let xml = XMLSerializer.serialize(response)
    
    do {
        try xml.write(
            to: URL(fileURLWithPath: pathStr),
            atomically: true,
            encoding: .utf8
        )
        return SRString("ok:\(pathStr)")
    } catch {
        return SRString("error:\(error.localizedDescription)")
    }
}

/// Start monitoring frontmost app changes.
///
/// Called from Rust: `swift!(fn claw_ax_start_frontmost_monitor(callback: *const c_void, dump_path: &SRString))`
@_cdecl("claw_ax_start_frontmost_monitor")
public func claw_ax_start_frontmost_monitor(
    _ callbackPtr: UnsafeRawPointer,
    dumpPath: SRString
) {
    FrontmostMonitor.start(callbackPtr: callbackPtr, dumpPath: dumpPath.toString())
}

/// Stop monitoring frontmost app changes.
///
/// Called from Rust: `swift!(fn claw_ax_stop_frontmost_monitor())`
@_cdecl("claw_ax_stop_frontmost_monitor")
public func claw_ax_stop_frontmost_monitor() {
    FrontmostMonitor.stop()
}
```

---

## Step 9: Update `Package.swift`

No changes needed! SPM automatically includes all `.swift` files under `Sources/ClawAccessibility/` regardless of subdirectory structure. The new files in `Core/`, `Models/`, etc. will be compiled as part of the same target.

The existing `Package.swift` already correctly specifies:
- Target name: `ClawAccessibility`
- Source path: `Sources/ClawAccessibility/` (implicit)
- Dependencies: `SwiftRs`, `ApplicationServices`, `AppKit`

---

## Testing the Swift Layer

### Manual Testing During Development

1. **Build and run**:
   ```bash
   cd src-tauri && cargo tauri dev
   ```

2. **Check Console.app** (or Tauri's terminal output) for `[ClawAccessibility]` log lines

3. **Test with different apps**:
   - Finder (native Cocoa, rich tree)
   - Safari (WebKit, deep web content tree)
   - VS Code (Electron, flat tree with web ARIA)
   - Terminal (simple tree)
   - Calculator (very small, clean tree — great for quick testing)

### Verifying JSON Output

After implementing the JSON path, you can test by calling the Tauri command from the browser console (in dev mode):

```javascript
const result = await window.__TAURI__.core.invoke('get_accessibility_tree');
console.log(JSON.stringify(result, null, 2));
```

Expected output structure:
```json
{
  "app": {
    "pid": 1234,
    "bundleIdentifier": "com.apple.calculator",
    "name": "Calculator"
  },
  "root": {
    "id": "n_0",
    "role": "AXApplication",
    "title": "Calculator",
    "children": [
      {
        "id": "n_1",
        "role": "AXWindow",
        "title": "Calculator",
        "children": [
          {
            "id": "n_2",
            "role": "AXGroup",
            "children": [
              {
                "id": "n_3",
                "role": "AXButton",
                "title": "1",
                "actions": ["AXPress"],
                "children": []
              }
            ]
          }
        ]
      }
    ]
  },
  "nodeCount": 47,
  "truncated": false
}
```

---

> **Next**: [04-rust-bridge-layer.md](./04-rust-bridge-layer.md) — Implementing the Rust side of the bridge.
