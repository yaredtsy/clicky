# 08 — Mouse-to-Tree Inspector (Hover on Screen → Select in Tree)

> **Goal**: Track the user's mouse position globally, find the AX element under the cursor, highlight it with the overlay, and select the corresponding node in the tree viewer — like Xcode's Accessibility Inspector "point inspection" mode.

---

## Table of Contents

1. [How It Works (High-Level)](#1-how-it-works-high-level)
2. [Why This Is Hard](#2-why-this-is-hard)
3. [Architecture Overview](#3-architecture-overview)
4. [Step 1: CGEvent Tap for Global Mouse Tracking (Swift)](#step-1-cgevent-tap-for-global-mouse-tracking-swift)
5. [Step 2: AXUIElementCopyElementAtPosition (Swift)](#step-2-axuielementcopyelementatposition-swift)
6. [Step 3: Build the InspectorMode Module (Swift)](#step-3-build-the-inspectormode-module-swift)
7. [Step 4: FFI Exports for Inspector Mode (Swift)](#step-4-ffi-exports-for-inspector-mode-swift)
8. [Step 5: Rust Bridge for Inspector Mode](#step-5-rust-bridge-for-inspector-mode)
9. [Step 6: Rust Commands and Event Emission](#step-6-rust-commands-and-event-emission)
10. [Step 7: React Integration — Inspector Mode Hook](#step-7-react-integration--inspector-mode-hook)
11. [Step 8: Finding the Matching Node in the Cached Tree](#step-8-finding-the-matching-node-in-the-cached-tree)
12. [Performance: Throttling Mouse Events](#performance-throttling-mouse-events)
13. [The Matching Problem — How to Identify the Same Node](#the-matching-problem--how-to-identify-the-same-node)
14. [Edge Cases](#edge-cases)
15. [Full Data Flow Diagram](#full-data-flow-diagram)
16. [Files Changed Summary](#files-changed-summary)

---

## 1. How It Works (High-Level)

```
User enters "Inspector Mode" (toggles a button in toolbar)
        │
        ▼
Swift installs a CGEvent tap → listens for mouse movement globally
        │
        ▼
On each mouse move (throttled to ~15fps):
   1. Get cursor position (CGPoint)
   2. Call AXUIElementCopyElementAtPosition(systemWide, x, y)
   3. Read the element's role, title, frame, and parent chain
   4. Serialize to JSON → call Rust callback
        │
        ▼
Rust emits Tauri event "ax-element-at-cursor" with element info
        │
        ▼
React receives event:
   1. Match the element to a node in the cached tree (by frame + role)
   2. Select that node in the tree (expand parents, scroll into view)
   3. Update the highlight overlay to show the element's frame
        │
        ▼
User sees: hovered element highlighted + tree scrolled to match
```

---

## 2. Why This Is Hard

This feature has three significant challenges:

### Challenge 1: Global Mouse Tracking

We need to track the mouse position even when our app is not in focus — when the user is hovering over Safari, Finder, etc. This requires a **CGEvent tap**, which is a low-level Quartz facility that intercepts input events system-wide.

**Requirement**: Accessibility permission (already have it).

### Challenge 2: Element Hit-Testing

`AXUIElementCopyElementAtPosition` is the API that finds the AX element at a screen point. But:
- It's an IPC call to the target app (can be slow: 1-10ms)
- At 60fps mouse movement, that's 60 IPC calls/second
- The target app may be unresponsive (hit-test blocks)
- It returns the *deepest* element at that point, which might be too specific

**Solution**: Throttle to ~15fps and run on a background queue with timeout.

### Challenge 3: Matching the Hit Element to Our Cached Tree

`AXUIElementCopyElementAtPosition` returns a *new* `AXUIElement` reference. This is NOT the same pointer as the one we traversed earlier. We need to match it to a node in our cached tree by comparing attributes.

**Approach**: Match by `(role × frame)` — if a node in our tree has the same role and identical frame as the hit-tested element, it's the same element. This works because within a single app, the combination of role + frame is nearly always unique.

---

## 3. Architecture Overview

```
┌── macOS System ─────────────────────────────────────────────────┐
│                                                                  │
│   CGEvent Tap (passive listener for mouse movement)              │
│       │                                                          │
│       │  Mouse position: (543.0, 312.0)                          │
│       ▼                                                          │
│   AXUIElementCopyElementAtPosition(systemWide, 543, 312)         │
│       │                                                          │
│       │  Returns: AXUIElement (button in Safari)                 │
│       ▼                                                          │
│   Read: role="AXButton", title="Downloads",                     │
│          frame={x:530, y:300, w:80, h:24}                        │
│       │                                                          │
└───────┼──────────────────────────────────────────────────────────┘
        │ C callback to Rust
        ▼
┌── Rust ─────────────────────────────────────────────────────────┐
│                                                                  │
│   app.emit("ax-element-at-cursor", {                             │
│     role: "AXButton",                                            │
│     title: "Downloads",                                          │
│     frame: { x: 530, y: 300, w: 80, h: 24 },                    │
│     pid: 12345                                                   │
│   })                                                             │
│                                                                  │
└───────┬──────────────────────────────────────────────────────────┘
        │ Tauri event
        ▼
┌── React ────────────────────────────────────────────────────────┐
│                                                                  │
│   1. Receive "ax-element-at-cursor" event                        │
│   2. Search cached tree for node matching (role + frame)          │
│   3. Select that node → tree expands to it, detail panel updates │
│   4. Highlight overlay moves to element's frame                  │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

---

## Step 1: CGEvent Tap for Global Mouse Tracking (Swift)

### What Is a CGEvent Tap?

A CGEvent tap is a low-level mechanism that intercepts input events (keyboard, mouse) before they reach their target application. We use a **passive** (listen-only) tap so we don't interfere with normal mouse behavior.

### Key Concepts

```swift
// Event tap location determines where in the event pipeline we listen:
// .cghidEventTap    — after HID (hardware) processing, before apps
// .cgSessionEventTap — after session-level processing
// .cgAnnotatedSessionEventTap — annotated events

// Tap placement:
// .headInsertEventTap — first in the chain (we see events first)
// .tailAppendEventTap — last in the chain (we see events after others)

// Options:
// .defaultTap     — can modify events (active tap)
// .listenOnly     — read-only, cannot modify events (what we want)
```

### The CGEvent Tap Implementation

```swift
import ApplicationServices

/// Install a CGEvent tap that monitors global mouse movement.
///
/// ## Requirements
/// - Accessibility permission (AXIsProcessTrusted must return true)
/// - Without permission, CGEvent.tapCreate returns nil
///
/// ## Why .listenOnly?
/// We only need to observe mouse position, not modify events.
/// A .defaultTap would require us to return the event (more work)
/// and could interfere with other apps if we have bugs.
///
/// ## Performance
/// The callback fires for EVERY mouse move event (60+ times/second).
/// We MUST throttle inside the callback to avoid overwhelming the
/// Accessibility API with hit-test queries.
func createMouseTracker(
    callback: @escaping (CGPoint) -> Void
) -> (tap: CFMachPort, source: CFRunLoopSource)? {
    
    // We want to listen for:
    // - mouseMoved: any mouse movement
    // - leftMouseDragged: mouse move during left-click drag
    // - rightMouseDragged: mouse move during right-click drag
    // - otherMouseDragged: mouse move during other button drag
    let eventMask: CGEventMask = (
        (1 << CGEventType.mouseMoved.rawValue) |
        (1 << CGEventType.leftMouseDragged.rawValue) |
        (1 << CGEventType.rightMouseDragged.rawValue) |
        (1 << CGEventType.otherMouseDragged.rawValue)
    )
    
    // Wrap callback in a context struct for the C function pointer
    // (We can't capture Swift closures in C callbacks directly)
    let context = Unmanaged.passRetained(
        MouseCallbackWrapper(callback: callback)
    ).toOpaque()
    
    guard let tap = CGEvent.tapCreate(
        tap: .cghidEventTap,           // Listen at HID level
        place: .headInsertEventTap,     // First in chain
        options: .listenOnly,           // Passive — don't modify events
        eventsOfInterest: eventMask,
        callback: mouseEventCallback,   // C-compatible callback
        userInfo: context               // Our wrapper
    ) else {
        print("[Inspector] Failed to create CGEvent tap (need Accessibility permission)")
        return nil
    }
    
    guard let source = CFMachPortCreateRunLoopSource(
        kCFAllocatorDefault, tap, 0
    ) else {
        print("[Inspector] Failed to create run loop source")
        return nil
    }
    
    return (tap, source)
}

/// C-compatible callback for the CGEvent tap.
///
/// This is called by the system for each mouse event. We:
/// 1. Extract the mouse position
/// 2. Forward it to the Swift closure wrapped in context
private func mouseEventCallback(
    proxy: CGEventTapProxy,
    type: CGEventType,
    event: CGEvent,
    userInfo: UnsafeMutableRawPointer?
) -> Unmanaged<CGEvent>? {
    
    // Handle tap being disabled (system can disable if it's too slow)
    if type == .tapDisabledByTimeout || type == .tapDisabledByUserInput {
        // Re-enable the tap
        if let userInfo = userInfo {
            let wrapper = Unmanaged<MouseCallbackWrapper>
                .fromOpaque(userInfo).takeUnretainedValue()
            if let tap = wrapper.tap {
                CGEvent.tapEnable(tap: tap, enable: true)
            }
        }
        return Unmanaged.passUnretained(event)
    }
    
    guard let userInfo = userInfo else {
        return Unmanaged.passUnretained(event)
    }
    
    let wrapper = Unmanaged<MouseCallbackWrapper>
        .fromOpaque(userInfo).takeUnretainedValue()
    
    let location = event.location  // CGPoint in screen coordinates
    wrapper.callback(location)
    
    // Return the event unmodified (required even for listen-only taps)
    return Unmanaged.passUnretained(event)
}

/// Wrapper to hold the Swift closure + tap reference.
/// Must be a class (reference type) for Unmanaged.
private class MouseCallbackWrapper {
    let callback: (CGPoint) -> Void
    var tap: CFMachPort?
    
    init(callback: @escaping (CGPoint) -> Void) {
        self.callback = callback
    }
}
```

### Why Not NSEvent.addGlobalMonitorForEvents?

```swift
// This is simpler but has limitations:
NSEvent.addGlobalMonitorForEvents(matching: .mouseMoved) { event in
    let point = NSEvent.mouseLocation  // AppKit coordinates (bottom-left origin)
    // ...
}
```

| Feature | NSEvent Global Monitor | CGEvent Tap |
|---------|----------------------|-------------|
| Coordinate system | AppKit (bottom-left origin) | Screen (top-left origin, matches AX) |
| Event types | Limited to NSEvent types | All event types |
| Performance | Higher overhead | Lower overhead (closer to hardware) |
| Reliability | Can miss events under load | More reliable |
| Permission | Accessibility required | Accessibility required |

We use CGEvent tap because:
1. Its coordinates match AX coordinates (top-left origin) — no conversion needed
2. It's more reliable under load
3. It's what Apple's own Accessibility Inspector uses internally

---

## Step 2: AXUIElementCopyElementAtPosition (Swift)

### The Hit-Test API

```swift
/// Find the accessibility element at a specific screen point.
///
/// This is the core API for mouse-based inspection. Given a screen
/// coordinate, it returns the deepest (most specific) AX element
/// at that position.
///
/// ## How It Works Internally
///
/// 1. macOS looks at the window z-order to find the topmost window at (x, y)
/// 2. It sends an IPC message to that window's owning process
/// 3. The process walks its view hierarchy to find the view at (x, y)
/// 4. The view's accessibility element is returned
///
/// ## Performance
///
/// This involves IPC to the target process. Typical latency:
/// - Responsive app: 0.5-2ms
/// - Busy app: 5-50ms
/// - Hung app: blocks until timeout (~5s)
///
/// ## Edge Cases
///
/// - Transparent areas: Returns the element beneath the transparent area
/// - Overlapping windows: Returns element in the topmost window
/// - Desktop: Returns the Finder's desktop element
/// - Our overlay panel: We set ignoresMouseEvents=true, so the hit-test
///   sees through our overlay to the app below (critical!)
func elementAtPosition(_ point: CGPoint) -> AXUIElement? {
    let systemWide = AXUIElementCreateSystemWide()
    var element: AXUIElement?
    
    let error = AXUIElementCopyElementAtPosition(
        systemWide,
        Float(point.x),  // Note: Float, not Double!
        Float(point.y),
        &element
    )
    
    guard error == .success, let element = element else {
        return nil
    }
    
    return element
}
```

### Reading the Hit Element's Info

```swift
/// Read essential info from an AX element for matching against the cached tree.
///
/// We read a minimal set of attributes (not a full traversal) since
/// this runs on every mouse move (~15fps).
struct HitElementInfo: Codable {
    let role: String
    let subrole: String?
    let title: String?
    let description: String?
    let value: String?
    let frame: FrameModel?
    let pid: Int32
    let actions: [String]
}

func readHitElementInfo(_ element: AXUIElement) -> HitElementInfo? {
    let role = axString(element, kAXRoleAttribute as String) ?? "AXUnknown"
    let subrole = axString(element, kAXSubroleAttribute as String)
    let title = axString(element, kAXTitleAttribute as String)
    let description = axString(element, kAXDescriptionAttribute as String)
    let value = axString(element, kAXValueAttribute as String)
    let frame = axFrame(element).map { FrameModel(from: $0) }
    let actions = axActionNames(element)
    
    // Get the PID of the process owning this element
    var pid: pid_t = 0
    AXUIElementGetPid(element, &pid)
    
    return HitElementInfo(
        role: role,
        subrole: subrole,
        title: title,
        description: description,
        value: value,
        frame: frame,
        pid: pid,
        actions: actions
    )
}
```

---

## Step 3: Build the InspectorMode Module (Swift)

### `Sources/ClawAccessibility/Inspector/InspectorMode.swift`

```swift
import ApplicationServices
import Foundation

/// Manages the "inspector mode" — global mouse tracking with AX hit-testing.
///
/// ## Lifecycle
///
/// 1. User clicks "Inspect" button in the UI
/// 2. Rust calls `claw_ax_start_inspector(callback)`
/// 3. Swift installs a CGEvent tap and starts listening
/// 4. On each mouse move (throttled):
///    a. Hit-test the element at cursor position
///    b. Serialize its info to JSON
///    c. Call the Rust callback with the JSON
/// 5. User clicks "Stop Inspect" or presses Escape
/// 6. Rust calls `claw_ax_stop_inspector()`
/// 7. Swift removes the CGEvent tap
///
/// ## Thread Safety
///
/// The CGEvent tap callback fires on the run loop thread.
/// We do the AX hit-test on that same thread (it's lightweight).
/// The Rust callback must be safe to call from any thread.
///
/// ## Throttling Strategy
///
/// Mouse moves fire at 60+ Hz. We throttle to ~15fps (67ms interval):
/// - Avoids overwhelming the target app with AX queries
/// - Still feels responsive to the user
/// - Reduces CPU usage from ~30% to ~5%
enum InspectorMode {
    
    typealias InspectorCallback = @convention(c) (UnsafePointer<CChar>?) -> Void
    
    // State
    private static var eventTap: CFMachPort?
    private static var runLoopSource: CFRunLoopSource?
    private static var callbackWrapper: MouseCallbackWrapper?
    private static var lastEmitTime: CFAbsoluteTime = 0
    private static var lastRole: String = ""
    private static var lastFrameKey: String = ""
    
    /// Minimum interval between hit-test queries (in seconds).
    /// 0.067s ≈ 15fps — good balance of responsiveness and CPU usage.
    private static let throttleInterval: CFAbsoluteTime = 0.067
    
    /// Start inspector mode.
    ///
    /// - Parameter callbackPtr: C function pointer from Rust to receive
    ///   JSON strings with hit element info.
    static func start(callbackPtr: UnsafeRawPointer) {
        // Stop any existing inspector session
        stop()
        
        let rustCallback = unsafeBitCast(callbackPtr, to: InspectorCallback.self)
        
        // Create the CGEvent tap with our throttled handler
        let wrapper = MouseCallbackWrapper { [rustCallback] point in
            handleMouseMove(at: point, callback: rustCallback)
        }
        
        guard let tap = CGEvent.tapCreate(
            tap: .cghidEventTap,
            place: .headInsertEventTap,
            options: .listenOnly,
            eventsOfInterest: CGEventMask(
                (1 << CGEventType.mouseMoved.rawValue) |
                (1 << CGEventType.leftMouseDragged.rawValue)
            ),
            callback: globalMouseCallback,
            userInfo: Unmanaged.passRetained(wrapper).toOpaque()
        ) else {
            print("[Inspector] Failed to create event tap — check Accessibility permission")
            return
        }
        
        wrapper.tap = tap
        
        guard let source = CFMachPortCreateRunLoopSource(kCFAllocatorDefault, tap, 0) else {
            print("[Inspector] Failed to create run loop source")
            return
        }
        
        // Add to the main run loop
        CFRunLoopAddSource(CFRunLoopGetMain(), source, .commonModes)
        CGEvent.tapEnable(tap: tap, enable: true)
        
        // Retain references
        eventTap = tap
        runLoopSource = source
        callbackWrapper = wrapper
        
        print("[Inspector] Started — tracking mouse globally")
    }
    
    /// Stop inspector mode. Remove the CGEvent tap.
    static func stop() {
        if let tap = eventTap {
            CGEvent.tapEnable(tap: tap, enable: false)
        }
        if let source = runLoopSource {
            CFRunLoopRemoveSource(CFRunLoopGetMain(), source, .commonModes)
        }
        if let wrapper = callbackWrapper {
            // Release the retained wrapper
            Unmanaged.passUnretained(wrapper).release()
        }
        eventTap = nil
        runLoopSource = nil
        callbackWrapper = nil
        lastRole = ""
        lastFrameKey = ""
        
        print("[Inspector] Stopped")
    }
    
    /// Whether inspector mode is currently active.
    static var isActive: Bool {
        return eventTap != nil
    }
    
    // MARK: - Private
    
    /// Handle a mouse move event.
    ///
    /// This is called from the CGEvent tap callback. We:
    /// 1. Throttle to ~15fps
    /// 2. Hit-test the AX element at the cursor position
    /// 3. Deduplicate (skip if same element as last time)
    /// 4. Serialize and send to Rust
    private static func handleMouseMove(
        at point: CGPoint,
        callback: InspectorCallback
    ) {
        // --- Throttle ---
        let now = CFAbsoluteTimeGetCurrent()
        guard now - lastEmitTime >= throttleInterval else { return }
        lastEmitTime = now
        
        // --- Hit-test ---
        guard let element = elementAtPosition(point) else {
            // No element at this position (desktop, transparent area)
            return
        }
        
        // --- Read element info ---
        guard let info = readHitElementInfo(element) else { return }
        
        // --- Deduplicate ---
        // If the cursor is still over the same element, don't re-emit.
        // We identify "same element" by role + frame. This avoids
        // sending redundant events when the mouse moves within the
        // same button/label.
        let frameKey: String
        if let f = info.frame {
            frameKey = "\(f.x),\(f.y),\(f.width),\(f.height)"
        } else {
            frameKey = "no-frame"
        }
        
        if info.role == lastRole && frameKey == lastFrameKey {
            return  // Same element, skip
        }
        lastRole = info.role
        lastFrameKey = frameKey
        
        // --- Serialize to JSON ---
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.sortedKeys]
        guard let data = try? encoder.encode(info),
              let json = String(data: data, encoding: .utf8) else {
            return
        }
        
        // --- Send to Rust ---
        json.withCString { ptr in
            guard let dup = strdup(ptr) else { return }
            callback(UnsafePointer(dup))
            free(dup)
        }
    }
    
    /// Find the AX element at a screen position.
    private static func elementAtPosition(_ point: CGPoint) -> AXUIElement? {
        let systemWide = AXUIElementCreateSystemWide()
        var element: AXUIElement?
        
        let error = AXUIElementCopyElementAtPosition(
            systemWide,
            Float(point.x),
            Float(point.y),
            &element
        )
        
        guard error == .success else { return nil }
        return element
    }
}

// MARK: - CGEvent Tap C Callback

/// C-compatible callback for the CGEvent tap.
/// Forwards the event to the Swift wrapper's closure.
private func globalMouseCallback(
    proxy: CGEventTapProxy,
    type: CGEventType,
    event: CGEvent,
    userInfo: UnsafeMutableRawPointer?
) -> Unmanaged<CGEvent>? {
    // Re-enable tap if it was disabled by the system
    if type == .tapDisabledByTimeout || type == .tapDisabledByUserInput {
        if let userInfo = userInfo {
            let wrapper = Unmanaged<MouseCallbackWrapper>
                .fromOpaque(userInfo).takeUnretainedValue()
            if let tap = wrapper.tap {
                CGEvent.tapEnable(tap: tap, enable: true)
            }
        }
        return Unmanaged.passUnretained(event)
    }
    
    guard let userInfo = userInfo else {
        return Unmanaged.passUnretained(event)
    }
    
    let wrapper = Unmanaged<MouseCallbackWrapper>
        .fromOpaque(userInfo).takeUnretainedValue()
    wrapper.callback(event.location)
    
    return Unmanaged.passUnretained(event)
}

/// Wrapper class to hold the Swift closure and tap reference.
private class MouseCallbackWrapper {
    let callback: (CGPoint) -> Void
    var tap: CFMachPort?
    
    init(callback: @escaping (CGPoint) -> Void) {
        self.callback = callback
    }
}
```

### Understanding `tapDisabledByTimeout`

macOS automatically disables a CGEvent tap if its callback takes too long (>~250ms). This is a safety mechanism — a buggy event tap could freeze all mouse input. When this happens:

1. The system fires a `tapDisabledByTimeout` event type
2. We need to manually re-enable the tap via `CGEvent.tapEnable(tap:enable:)`
3. If we don't, the inspector silently stops working

Our throttling (skipping calls faster than 67ms) and lightweight processing (single AX hit-test) make timeout unlikely but we handle it defensively.

---

## Step 4: FFI Exports for Inspector Mode (Swift)

### Add to `FFI/Exports.swift`

```swift
/// Start the mouse inspector mode.
///
/// Installs a CGEvent tap for global mouse tracking and calls back
/// to Rust with JSON info about the AX element under the cursor.
///
/// Called from Rust:
///   swift!(fn claw_ax_start_inspector(callback: *const c_void))
@_cdecl("claw_ax_start_inspector")
public func claw_ax_start_inspector(_ callbackPtr: UnsafeRawPointer) {
    InspectorMode.start(callbackPtr: callbackPtr)
}

/// Stop the mouse inspector mode.
///
/// Removes the CGEvent tap and stops all mouse tracking.
///
/// Called from Rust: swift!(fn claw_ax_stop_inspector())
@_cdecl("claw_ax_stop_inspector")
public func claw_ax_stop_inspector() {
    InspectorMode.stop()
}

/// Check if inspector mode is currently active.
///
/// Called from Rust: swift!(fn claw_ax_is_inspector_active() -> Bool)
@_cdecl("claw_ax_is_inspector_active")
public func claw_ax_is_inspector_active() -> Bool {
    InspectorMode.isActive
}
```

---

## Step 5: Rust Bridge for Inspector Mode

### Add to `src-tauri/src/bridge/swift_ffi.rs`

```rust
// Inspector mode
swift!(fn claw_ax_start_inspector(callback: *const c_void));
swift!(fn claw_ax_stop_inspector());
swift!(fn claw_ax_is_inspector_active() -> Bool);
```

### Add to `src-tauri/src/bridge/mod.rs`

```rust
/// Start inspector mode (global mouse tracking + AX hit-testing).
pub fn start_inspector(callback: extern "C" fn(*const std::ffi::c_char)) {
    unsafe {
        swift_ffi::claw_ax_start_inspector(callback as *const c_void);
    }
}

/// Stop inspector mode.
pub fn stop_inspector() {
    unsafe { swift_ffi::claw_ax_stop_inspector() }
}

/// Check if inspector mode is active.
pub fn is_inspector_active() -> bool {
    unsafe { swift_ffi::claw_ax_is_inspector_active() }
}
```

---

## Step 6: Rust Commands and Event Emission

### `src-tauri/src/commands/inspector.rs`

```rust
//! Tauri commands for mouse inspector mode.

use std::ffi::{c_char, CStr};
use tauri::Emitter;

/// Start the mouse inspector mode.
///
/// When active, the inspector tracks the mouse position globally,
/// hit-tests the AX element under the cursor, and emits
/// "ax-element-at-cursor" events with the element's info.
///
/// ## Frontend Usage
/// ```typescript
/// await invoke('start_inspector_mode');
/// const unlisten = await listen<HitElementInfo>('ax-element-at-cursor', (e) => {
///   // e.payload has: { role, title, frame, pid, ... }
/// });
/// ```
#[tauri::command]
pub fn start_inspector_mode(app: tauri::AppHandle) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        if !crate::bridge::is_process_trusted(false) {
            return Err("Accessibility permission required for inspector mode.".into());
        }
        crate::state::set_handle(app);
        crate::bridge::start_inspector(inspector_callback);
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = app;
        Err("Inspector mode is only available on macOS.".into())
    }
}

/// Stop the mouse inspector mode.
///
/// ## Frontend Usage
/// ```typescript
/// await invoke('stop_inspector_mode');
/// ```
#[tauri::command]
pub fn stop_inspector_mode() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        crate::bridge::stop_inspector();
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err("Inspector mode is only available on macOS.".into())
    }
}

/// Check if inspector mode is currently active.
#[tauri::command]
pub fn is_inspector_active() -> Result<bool, String> {
    #[cfg(target_os = "macos")]
    {
        Ok(crate::bridge::is_inspector_active())
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(false)
    }
}

/// C callback invoked by Swift when the element under the cursor changes.
///
/// Receives a JSON string with the hit element's info.
/// Emits "ax-element-at-cursor" Tauri event to the frontend.
#[cfg(target_os = "macos")]
extern "C" fn inspector_callback(json_c: *const c_char) {
    if json_c.is_null() { return; }

    let json = unsafe { CStr::from_ptr(json_c) }
        .to_string_lossy()
        .into_owned();

    // Parse JSON to validate it and emit as a structured event
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&json) {
        crate::state::with_handle(|app| {
            let _ = app.emit("ax-element-at-cursor", value);
        });
    }
}
```

### Register in `commands/mod.rs`

```rust
pub mod inspector;

pub use inspector::{start_inspector_mode, stop_inspector_mode, is_inspector_active};
```

### Register in `lib.rs`

```rust
.invoke_handler(tauri::generate_handler![
    // ... existing commands ...
    // Inspector
    commands::start_inspector_mode,
    commands::stop_inspector_mode,
    commands::is_inspector_active,
])
```

---

## Step 7: React Integration — Inspector Mode Hook

### `src/models/hit-element.ts`

```typescript
import type { Frame } from './ax-tree';

/** Info about the AX element currently under the mouse cursor. */
export interface HitElementInfo {
  role: string;
  subrole?: string;
  title?: string;
  description?: string;
  value?: string;
  frame?: Frame;
  pid: number;
  actions: string[];
}
```

### `src/hooks/useInspectorMode.ts`

```typescript
import { useCallback, useEffect, useState, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { AXNode } from '../models';
import type { HitElementInfo } from '../models/hit-element';
import { highlightElement, clearHighlight } from '../services/accessibility';
import { findNodeByRoleAndFrame } from '../utils/tree-utils';

interface UseInspectorModeResult {
  /** Whether inspector mode is active. */
  active: boolean;
  /** The element currently under the cursor. */
  hoveredElement: HitElementInfo | null;
  /** The tree node matching the hovered element (if found in cached tree). */
  matchedNodeId: string | null;
  /** Start inspector mode. */
  start: () => Promise<void>;
  /** Stop inspector mode. */
  stop: () => Promise<void>;
  /** Toggle inspector mode. */
  toggle: () => Promise<void>;
}

/**
 * Hook to manage the mouse inspector mode.
 *
 * When active:
 * 1. Listens for "ax-element-at-cursor" events from the backend
 * 2. Matches the element to a node in the cached tree
 * 3. Updates the highlight overlay
 * 4. Provides the matched node ID for tree selection
 *
 * @param tree - The cached AX tree (from useAccessibilityTree)
 * @param onSelectNode - Callback to select a node in the tree
 */
export function useInspectorMode(
  tree: AXNode | null,
  onSelectNode: (node: AXNode) => void
): UseInspectorModeResult {
  const [active, setActive] = useState(false);
  const [hoveredElement, setHoveredElement] = useState<HitElementInfo | null>(null);
  const [matchedNodeId, setMatchedNodeId] = useState<string | null>(null);
  const treeRef = useRef(tree);
  
  // Keep treeRef current without re-running effects
  useEffect(() => { treeRef.current = tree; }, [tree]);

  const start = useCallback(async () => {
    try {
      await invoke('start_inspector_mode');
      setActive(true);
    } catch (e) {
      console.error('Failed to start inspector:', e);
    }
  }, []);

  const stop = useCallback(async () => {
    try {
      await invoke('stop_inspector_mode');
      setActive(false);
      setHoveredElement(null);
      setMatchedNodeId(null);
      await clearHighlight();
    } catch (e) {
      console.error('Failed to stop inspector:', e);
    }
  }, []);

  const toggle = useCallback(async () => {
    if (active) {
      await stop();
    } else {
      await start();
    }
  }, [active, start, stop]);

  // Listen for element-at-cursor events when active
  useEffect(() => {
    if (!active) return;

    let unlisten: UnlistenFn | undefined;

    void listen<HitElementInfo>('ax-element-at-cursor', (event) => {
      const element = event.payload;
      setHoveredElement(element);

      // Highlight the element on screen
      if (element.frame) {
        void highlightElement(element.frame);
      }

      // Match to a node in the cached tree
      if (treeRef.current && element.frame) {
        const matched = findNodeByRoleAndFrame(
          treeRef.current,
          element.role,
          element.frame
        );
        if (matched) {
          setMatchedNodeId(matched.id);
          onSelectNode(matched);
        } else {
          setMatchedNodeId(null);
        }
      }
    }).then((fn) => { unlisten = fn; });

    return () => {
      unlisten?.();
    };
  }, [active, onSelectNode]);

  // Stop inspector on unmount
  useEffect(() => {
    return () => {
      if (active) {
        void invoke('stop_inspector_mode');
      }
    };
  }, [active]);

  return { active, hoveredElement, matchedNodeId, start, stop, toggle };
}
```

### Toolbar Integration

Add an "Inspect" toggle button to the toolbar:

```tsx
// In Toolbar.tsx
<button
  className={`toolbar__button ${inspectorActive ? 'toolbar__button--active' : ''}`}
  onClick={onToggleInspector}
  title={inspectorActive ? 'Stop inspector (Esc)' : 'Inspect element under cursor'}
>
  {inspectorActive ? '🎯 Inspecting...' : '🔍 Inspect'}
</button>
```

### Keyboard Shortcut: Escape to Stop

```typescript
// In App.tsx, add keyboard listener
useEffect(() => {
  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === 'Escape' && inspectorMode.active) {
      void inspectorMode.stop();
    }
  };
  window.addEventListener('keydown', handleKeyDown);
  return () => window.removeEventListener('keydown', handleKeyDown);
}, [inspectorMode.active, inspectorMode.stop]);
```

---

## Step 8: Finding the Matching Node in the Cached Tree

### `src/utils/tree-utils.ts` — Add Matching Functions

```typescript
import type { AXNode, Frame } from '../models';

/**
 * Find a node in the tree that matches by role AND frame.
 *
 * ## Why role + frame?
 *
 * We can't use pointer identity (the AXUIElement from hit-testing
 * is a different object than the one we traversed). We need to
 * match by observable properties:
 *
 * - Role alone: Too many duplicates ("AXButton" appears 20+ times)
 * - Title alone: Many elements have no title
 * - Frame alone: Different elements could overlap (layered views)
 * - Role + Frame: Nearly unique within a single app.
 *   Two elements rarely have the same role AND exact same pixel position/size.
 *
 * ## Tolerance
 *
 * We use a small tolerance (0.5px) for frame comparison because:
 * - AX API returns Float32, we convert to Float64
 * - Rounding differences between traversal and hit-test
 * - Retina scaling can introduce sub-pixel differences
 */
export function findNodeByRoleAndFrame(
  root: AXNode,
  role: string,
  frame: Frame,
  tolerance: number = 0.5
): AXNode | null {
  // Check this node
  if (
    root.role === role &&
    root.frame &&
    framesMatch(root.frame, frame, tolerance)
  ) {
    return root;
  }

  // Recurse into children (DFS — returns first match)
  for (const child of root.children) {
    const found = findNodeByRoleAndFrame(child, role, frame, tolerance);
    if (found) return found;
  }

  return null;
}

/**
 * Compare two frames with a pixel tolerance.
 */
function framesMatch(a: Frame, b: Frame, tolerance: number): boolean {
  return (
    Math.abs(a.x - b.x) <= tolerance &&
    Math.abs(a.y - b.y) <= tolerance &&
    Math.abs(a.width - b.width) <= tolerance &&
    Math.abs(a.height - b.height) <= tolerance
  );
}

/**
 * Find a node and expand its parent chain in the tree.
 * Returns the IDs of all ancestors that should be expanded
 * for the node to be visible.
 */
export function getAncestorIds(root: AXNode, targetId: string): string[] {
  const path: string[] = [];

  function walk(node: AXNode): boolean {
    if (node.id === targetId) return true;
    for (const child of node.children) {
      if (walk(child)) {
        path.push(node.id);
        return true;
      }
    }
    return false;
  }

  walk(root);
  return path;
}
```

---

## Performance: Throttling Mouse Events

### The Problem

```
Mouse movement:    ████████████████████████████████  (60 events/sec)
AX hit-test:       ██░░██░░██░░██░░██░░██░░██░░██░░  (~30ms each)
Without throttle:  30% CPU, 60 IPC calls/sec to target app
With throttle:     5% CPU,  15 IPC calls/sec
```

### Our Throttling Strategy (In Swift)

```swift
private static let throttleInterval: CFAbsoluteTime = 0.067  // ~15fps

private static func handleMouseMove(at point: CGPoint, ...) {
    let now = CFAbsoluteTimeGetCurrent()
    guard now - lastEmitTime >= throttleInterval else { return }
    lastEmitTime = now
    // ... do the hit-test
}
```

### Deduplication (Also In Swift)

Even at 15fps, if the mouse is stationary (or moving within the same element), we'd send identical events. We deduplicate by comparing `role + frame`:

```swift
if info.role == lastRole && frameKey == lastFrameKey {
    return  // Same element, skip
}
```

### Combined Result

```
Raw events:   60/sec
After throttle: 15/sec  
After dedup:    2-5/sec (only when cursor moves to a NEW element)
```

This makes the feature feel responsive while keeping CPU usage low.

---

## The Matching Problem — How to Identify the Same Node

### Why Not Use `AXUIElement` Equality?

```swift
// This doesn't work reliably across separate queries:
let elem1 = // from traversal
let elem2 = // from hit-test at same position
CFEqual(elem1, elem2)  // Might be true, might be false!
```

`CFEqual` on `AXUIElement` compares the *internal representation*, which sometimes matches and sometimes doesn't depending on the target app's implementation.

### Our Strategy: Role + Frame Matching

```
Hit-test returns: role="AXButton", frame={x:340, y:410, w:52, h:40}
                                            │
Search cached tree for a node with ─────────┘
the same role AND frame
                                            │
Found: node id="n_42", role="AXButton",  ───┘
       frame={x:340, y:410, w:52, h:40}
```

### When Role + Frame Fails

| Scenario | Problem | Mitigation |
|----------|---------|------------|
| Overlapping elements | Two elements with same role at same position | Add title/value to the match criteria |
| Frame changed since traversal | User scrolled or resized | Re-traverse the tree on match failure |
| No frame | Some elements lack position info | Fall back to role + title matching |
| Identical siblings | Two buttons with same role and same size at same position | Extremely rare; accept closest match |

### Enhanced Matching (If Needed)

```typescript
function findNodeMultiCriteria(
  root: AXNode,
  role: string,
  frame: Frame,
  title?: string,
  value?: string
): AXNode | null {
  // Score-based matching: more matching attributes = higher score
  let bestMatch: AXNode | null = null;
  let bestScore = 0;

  function score(node: AXNode): number {
    let s = 0;
    if (node.role === role) s += 10;
    if (node.frame && framesMatch(node.frame, frame, 0.5)) s += 20;
    if (title && node.title === title) s += 5;
    if (value && node.value === value) s += 3;
    return s;
  }

  function walk(node: AXNode) {
    const s = score(node);
    if (s > bestScore) {
      bestScore = s;
      bestMatch = node;
    }
    for (const child of node.children) walk(child);
  }

  walk(root);
  return bestScore >= 30 ? bestMatch : null; // Require role + frame minimum
}
```

---

## Edge Cases

### 1. Hovering Over Our Own App

When the cursor moves over our Inspector window:
- The hit-test returns elements from our own app
- We should ignore these (same PID as us)

```typescript
// In the event handler
if (element.pid === currentAppPid) return; // Skip our own app
```

Get the current PID from the initial app setup and store it.

### 2. Target App Quits During Inspection

The hit-test returns `nil` for areas with no element. The inspector gracefully shows nothing.

### 3. Full-Screen Apps and Spaces

CGEvent taps work across all Spaces and full-screen apps. The overlay panel (with `can_join_all_spaces`) follows.

### 4. Cursor on Desktop (Finder)

`AXUIElementCopyElementAtPosition` on the desktop returns Finder's desktop element. This is valid — we'd show the Finder's tree with the desktop element selected.

### 5. System UI Elements (Menu Bar, Dock)

These are owned by system processes (WindowServer, Dock). Hit-testing works but the tree may be sparse or require special handling.

### 6. CGEvent Tap Disabled by System

macOS disables event taps that take too long (>250ms) or process too many events. We handle `tapDisabledByTimeout` by re-enabling it:

```swift
if type == .tapDisabledByTimeout {
    CGEvent.tapEnable(tap: tap, enable: true)
}
```

---

## Full Data Flow Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Inspector Mode Flow                        │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  [User clicks "Inspect" button]                                     │
│       │                                                             │
│       ▼                                                             │
│  invoke("start_inspector_mode")                                     │
│       │                                                             │
│       ▼                                                             │
│  Rust: bridge::start_inspector(callback)                            │
│       │                                                             │
│       ▼                                                             │
│  Swift: InspectorMode.start()                                       │
│       │                                                             │
│       ▼                                                             │
│  CGEvent.tapCreate() → installed on main run loop                   │
│       │                                                             │
│       │  ┌──────── Main Loop ────────────────────────────┐          │
│       │  │                                                │          │
│       │  │  [Mouse moves to position (543, 312)]          │          │
│       │  │       │                                        │          │
│       │  │       ▼                                        │          │
│       │  │  Throttle check: has 67ms passed? ──No──→ skip │          │
│       │  │       │ Yes                                    │          │
│       │  │       ▼                                        │          │
│       │  │  AXUIElementCopyElementAtPosition(543, 312)    │          │
│       │  │       │                                        │          │
│       │  │       ▼                                        │          │
│       │  │  Read: role, title, frame, pid                 │          │
│       │  │       │                                        │          │
│       │  │       ▼                                        │          │
│       │  │  Dedup check: same element? ──Yes──→ skip      │          │
│       │  │       │ No (new element)                       │          │
│       │  │       ▼                                        │          │
│       │  │  JSON encode → callback(json_ptr)              │          │
│       │  │       │                                        │          │
│       │  └───────┼────────────────────────────────────────┘          │
│       │          │                                                   │
│       │          ▼                                                   │
│       │  Rust: inspector_callback()                                  │
│       │     app.emit("ax-element-at-cursor", json)                   │
│       │          │                                                   │
│       │          ▼                                                   │
│       │  React: listen("ax-element-at-cursor")                       │
│       │     1. highlightElement(element.frame) → overlay moves       │
│       │     2. findNodeByRoleAndFrame(tree, role, frame)             │
│       │     3. onSelectNode(matched) → tree highlights + scrolls     │
│       │                                                             │
│  [User presses Escape]                                              │
│       │                                                             │
│       ▼                                                             │
│  invoke("stop_inspector_mode")                                      │
│       │                                                             │
│       ▼                                                             │
│  Swift: InspectorMode.stop() → CGEvent tap removed                  │
│  React: clearHighlight() → overlay hidden                           │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Files Changed Summary

### Swift (New Files)

| File | Purpose |
|------|---------|
| `Sources/ClawAccessibility/Inspector/InspectorMode.swift` | CGEvent tap, hit-testing, throttling, dedup |
| `Sources/ClawAccessibility/Models/HitElementInfo.swift` | Codable model for hit-test results |

### Swift (Modified Files)

| File | Changes |
|------|---------|
| `FFI/Exports.swift` | Add `claw_ax_start_inspector`, `claw_ax_stop_inspector`, `claw_ax_is_inspector_active` |

### Rust (New Files)

| File | Purpose |
|------|---------|
| `src-tauri/src/commands/inspector.rs` | `start_inspector_mode`, `stop_inspector_mode`, `is_inspector_active` commands + callback |

### Rust (Modified Files)

| File | Changes |
|------|---------|
| `bridge/swift_ffi.rs` | Add inspector FFI declarations |
| `bridge/mod.rs` | Add safe wrappers |
| `commands/mod.rs` | Register inspector commands |
| `lib.rs` | Add to `invoke_handler` |

### React (New Files)

| File | Purpose |
|------|---------|
| `src/models/hit-element.ts` | `HitElementInfo` interface |
| `src/hooks/useInspectorMode.ts` | Inspector mode state + event handling |

### React (Modified Files)

| File | Changes |
|------|---------|
| `src/utils/tree-utils.ts` | Add `findNodeByRoleAndFrame`, `getAncestorIds`, `framesMatch` |
| `src/components/toolbar/Toolbar.tsx` | Add Inspect toggle button |
| `src/App.tsx` | Wire up `useInspectorMode`, add Escape key handler |

---

> **Previous**: [07-tree-to-app-highlight.md](./07-tree-to-app-highlight.md) — The forward direction: click tree node → highlight on screen.
