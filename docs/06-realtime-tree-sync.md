# 06 — Real-Time Tree Sync

> **Goal**: Understand and implement the event system that keeps the tree viewer updated as the user switches between apps.

---

## Table of Contents

1. [Event Architecture Overview](#1-event-architecture-overview)
2. [The Full Event Chain](#2-the-full-event-chain)
3. [NSWorkspace Notifications (Swift)](#3-nsworkspace-notifications-swift)
4. [C Callback to Rust](#4-c-callback-to-rust)
5. [Tauri Event Emission (Rust)](#5-tauri-event-emission-rust)
6. [React Event Listener](#6-react-event-listener)
7. [When to Refresh vs When to Re-traverse](#7-when-to-refresh-vs-when-to-retraverse)
8. [Debouncing and Throttling](#8-debouncing-and-throttling)
9. [Handling Edge Cases](#9-handling-edge-cases)
10. [Future: Incremental Updates](#10-future-incremental-updates)

---

## 1. Event Architecture Overview

```
macOS System ──→ Swift Observer ──→ C Callback ──→ Rust Handler ──→ Tauri Event ──→ React Listener
                                                                                        │
                                                                                  Re-fetch tree
                                                                                  via invoke()
```

This is a **push-then-pull** model:
- **Push**: The system pushes "frontmost app changed" events all the way to React
- **Pull**: React then pulls the new tree by calling `get_accessibility_tree`

### Why Not Push the Tree Directly?

We could have Swift serialize the new tree and push it through the event chain. We don't because:

1. **Event payload size**: Tauri events should be lightweight (a bundle ID string vs a 500KB JSON tree)
2. **Backpressure**: If the user rapidly switches apps, we want to skip intermediate trees. With pull, React's `useEffect` cleanup naturally handles this.
3. **Decoupling**: The monitor (push) and tree extraction (pull) are independent concerns. The monitor could be useful without tree extraction (just tracking which app is active).

---

## 2. The Full Event Chain

### Step-by-Step Walkthrough

```
┌──────────────────────────────────────────────────────────────────┐
│ 1. USER switches to Safari (Cmd+Tab)                             │
└───────┬──────────────────────────────────────────────────────────┘
        │
        ▼
┌──────────────────────────────────────────────────────────────────┐
│ 2. macOS posts NSWorkspace.didActivateApplicationNotification    │
│    userInfo: { NSWorkspaceApplicationKey: NSRunningApplication } │
└───────┬──────────────────────────────────────────────────────────┘
        │
        ▼
┌──────────────────────────────────────────────────────────────────┐
│ 3. Swift observer fires (FrontmostMonitor.swift)                 │
│    - Checks activationPolicy == .regular ✓                       │
│    - Optionally dumps AX XML to file                             │
│    - Calls Rust callback with "com.apple.Safari"                │
└───────┬──────────────────────────────────────────────────────────┘
        │ C function pointer call
        ▼
┌──────────────────────────────────────────────────────────────────┐
│ 4. Rust callback (frontmost_changed_callback)                    │
│    - Reads bundle ID from C string                               │
│    - Gets AppHandle from global state                            │
│    - Calls app.emit("ax-frontmost-changed", "com.apple.Safari") │
└───────┬──────────────────────────────────────────────────────────┘
        │ Tauri IPC
        ▼
┌──────────────────────────────────────────────────────────────────┐
│ 5. React listener (useAccessibilityTree)                         │
│    - Receives event with payload "com.apple.Safari"              │
│    - Calls refresh() → invoke("get_accessibility_tree")          │
│    - Sets new tree state → component re-renders                  │
│    - Old selection is cleared (IDs change between traversals)    │
└──────────────────────────────────────────────────────────────────┘
```

---

## 3. NSWorkspace Notifications (Swift)

### Which Notifications and Why

We observe two notifications:

```swift
// 1. App activated (switched to)
NSWorkspace.didActivateApplicationNotification
// Fires when: user clicks on app, Cmd+Tab, app comes to foreground
// userInfo: NSRunningApplication of the newly active app

// 2. App launched
NSWorkspace.didLaunchApplicationNotification
// Fires when: a new app starts
// userInfo: NSRunningApplication of the new app
```

### Why Not `didDeactivateApplicationNotification`?

We don't need deactivate because:
- An activate always follows a deactivate
- We only care about the *new* frontmost app
- Processing deactivate + activate would double our work

### Why `.main` Queue?

```swift
center.addObserver(forName: ..., object: nil, queue: .main) { note in ... }
```

Swift callbacks to Rust (`@convention(c)`) must happen on the main thread because:
- `swift-rs` assumes main-thread execution
- `NSWorkspace` delivers notifications on `.main` by default anyway

---

## 4. C Callback to Rust

### Why C Function Pointer?

Swift's `@convention(c)` function type maps to Rust's `extern "C" fn`. We can't use closures because:
- Closures have unknown size (they capture variables)
- C function pointers have a fixed size (one pointer)
- `swift-rs` passes callbacks as `*const c_void`

### The Callback Signature

```swift
// Swift side — receives a C function pointer
typealias FrontCb = @convention(c) (UnsafePointer<CChar>?) -> Void

// Rust side — the actual callback function
extern "C" fn frontmost_changed_callback(bundle_c: *const c_char) { ... }
```

### Memory Management

```swift
payload.withCString { ptr in
    guard let dup = strdup(ptr) else { return }  // Allocate a copy
    callback(UnsafePointer(dup))                  // Pass to Rust
    free(dup)                                     // Free after Rust returns
}
```

Why `strdup`?
- `withCString` provides a temporary pointer that's only valid inside the closure
- But the callback might need the string beyond the closure's scope
- `strdup` creates a heap-allocated copy that lives until `free()`
- Since the callback is synchronous (it completes before we `free`), this is safe

---

## 5. Tauri Event Emission (Rust)

```rust
extern "C" fn frontmost_changed_callback(bundle_c: *const c_char) {
    if bundle_c.is_null() { return; }
    
    let bundle_id = unsafe { CStr::from_ptr(bundle_c) }
        .to_string_lossy()     // Handles non-UTF8 gracefully
        .into_owned();          // Creates an owned String

    crate::state::with_handle(|app| {
        let _ = app.emit("ax-frontmost-changed", &bundle_id);
        //       ^^^^                              ^^^^^^^^^
        //       Tauri AppHandle                   Payload (serialized to JSON)
    });
}
```

### How `app.emit()` Works

1. Tauri serializes the payload to JSON (a string becomes a JSON string `"com.apple.Safari"`)
2. Tauri sends the event through its IPC channel to the webview
3. The webview's `@tauri-apps/api/event` listener receives it
4. The listener deserializes the payload

### Event Name Convention

We prefix all custom events with `ax-` to namespace them:
- `ax-frontmost-changed` — frontmost app changed
- `ax-tree-updated` (future) — tree data pushed directly
- `ax-element-hovered` (future) — mouse hovering over an element

---

## 6. React Event Listener

### In the `useAccessibilityTree` Hook

```typescript
useEffect(() => {
  if (!monitoring) return;

  let unlisten: (() => void) | undefined;
  
  void onFrontmostChanged((_bundleId) => {
    void refresh();
  }).then((fn) => {
    unlisten = fn;
  });

  return () => {
    unlisten?.();
  };
}, [monitoring, refresh]);
```

### Critical Pattern: Cleanup on Unmount

```typescript
return () => {
  unlisten?.();  // Remove the event listener when the effect re-runs or component unmounts
};
```

Without this, you'd accumulate listeners and trigger multiple tree fetches per event.

### Why `void` Before Async Calls?

```typescript
void onFrontmostChanged(...).then(...);
void refresh();
```

The `void` operator explicitly marks that we're ignoring the promise. This:
- Prevents ESLint's `no-floating-promises` warning
- Communicates intent: "we know this is async, we don't need to await it"
- Is safer than `.catch(() => {})` which swallows errors silently

---

## 7. When to Refresh vs When to Re-traverse

### Current Approach: Full Re-traverse on Every App Switch

Every time the frontmost app changes, we:
1. Get the new app's PID
2. Create a fresh `AXUIElementCreateApplication(pid)`
3. Do a full DFS traversal
4. Build the entire `AXNodeModel` tree
5. Serialize to JSON
6. Send to React

This is the simplest approach and is correct. Full traversal of most apps takes 100-500ms.

### Future Optimization: Differential Updates

For apps with very large trees (VS Code with many tabs, Xcode with a big storyboard), we could:

1. **Cache the previous tree** in Swift
2. **Traverse again** and compare
3. **Send only the diff** (changed/added/removed nodes)
4. **Apply the diff in React** instead of replacing the whole tree

This is complex and not needed for V1. We'll implement it if profiling shows the full traversal is a bottleneck.

---

## 8. Debouncing and Throttling

### Problem: Rapid App Switching

If the user rapidly Cmd+Tabs through multiple apps, we'll fire many traversals. Each takes 100-500ms. They'll pile up.

### Solution: Debounce in React

```typescript
// In useAccessibilityTree.ts — enhanced version

import { useRef } from 'react';

// Inside the hook:
const debounceTimer = useRef<ReturnType<typeof setTimeout>>();

const debouncedRefresh = useCallback(() => {
  clearTimeout(debounceTimer.current);
  debounceTimer.current = setTimeout(() => {
    void refresh();
  }, 200); // Wait 200ms of quiet before refreshing
}, [refresh]);

// Use debouncedRefresh in the event listener
useEffect(() => {
  if (!monitoring) return;
  let unlisten: (() => void) | undefined;
  void onFrontmostChanged(() => {
    debouncedRefresh();
  }).then((fn) => { unlisten = fn; });
  return () => {
    unlisten?.();
    clearTimeout(debounceTimer.current);
  };
}, [monitoring, debouncedRefresh]);
```

### Why 200ms?

- Cmd+Tab cycling through apps fires at ~100ms intervals
- 200ms debounce means we only traverse the *final* app the user lands on
- If the user switches and stays for 200ms, they probably intend to inspect it

---

## 9. Handling Edge Cases

### Edge Case 1: App Quits While We're Inspecting

If the frontmost app quits, `NSWorkspace.frontmostApplication` changes to another app. Our monitor fires and we traverse the new app. No special handling needed.

If we're mid-traversal and the app quits, `AXUIElementCopyAttributeValue` returns `.invalidUIElement`. Our traversal handles this gracefully (returns nil for attributes, empty children).

### Edge Case 2: Our App Becomes Frontmost

When the user clicks on our Inspector window, our app becomes frontmost. We filter this out in Swift:

```swift
// In the did-launch observer
guard app.processIdentifier != ProcessInfo.processInfo.processIdentifier else { return }
```

But we should also handle it in the activate observer. Otherwise, clicking our window shows our own AX tree (which is meta but not useful). Add this guard:

```swift
// In handleAppChange
guard app.processIdentifier != ProcessInfo.processInfo.processIdentifier else { return }
```

### Edge Case 3: Full-Screen App

When the frontmost app is in full-screen, its AX tree is still accessible. The `AXFrame` coordinates change to fill the screen, but the tree structure is the same.

### Edge Case 4: Multiple Displays

`AXFrame` coordinates on secondary displays have offsets based on the display arrangement in Display Settings. The origin is still the primary display's top-left. Frames on a display to the left have negative X values.

### Edge Case 5: Permission Revoked While Running

If the user revokes Accessibility permission while the inspector is running:
- `AXUIElementCopyAttributeValue` calls start returning `.apiDisabled`
- Our traversal produces a tree with "AXUnknown" roles and no children
- The UI shows a sparse/broken tree
- We should detect this and show a re-permission prompt

Detection:
```swift
// In the traversal, check for apiDisabled
if status == .apiDisabled {
    // Permission was revoked — abort traversal
    return AXNodeModel(role: "AXPermissionRevoked", ...)
}
```

---

## 10. Future: Incremental Updates

### AX Notifications (for V2+)

macOS provides `AXObserver` for subscribing to element-level changes:

```swift
// Create an observer
var observer: AXObserver?
AXObserverCreate(pid, observerCallback, &observer)

// Register for specific notifications  
AXObserverAddNotification(observer, element, kAXFocusedUIElementChangedNotification, nil)
AXObserverAddNotification(observer, element, kAXValueChangedNotification, nil)
AXObserverAddNotification(observer, element, kAXUIElementDestroyedNotification, nil)

// Add to run loop
CFRunLoopAddSource(
    CFRunLoopGetMain(),
    AXObserverGetRunLoopSource(observer),
    .defaultMode
)
```

Available notifications:
| Notification | When It Fires |
|---|---|
| `kAXFocusedUIElementChangedNotification` | Focus moved to a different element |
| `kAXValueChangedNotification` | Element's value attribute changed |
| `kAXUIElementDestroyedNotification` | Element was removed from the UI |
| `kAXCreatedNotification` | New element was added |
| `kAXWindowMovedNotification` | Window was dragged |
| `kAXWindowResizedNotification` | Window was resized |
| `kAXSelectedChildrenChangedNotification` | Selection changed in a list/table |

This would allow us to update specific nodes in the tree instead of re-traversing the entire tree. However, it adds significant complexity:
- Need to maintain element references (can go stale)
- Need to sync observer lifecycle with the monitor
- Need a diffing algorithm in React

**This is a V2 feature.** For V1, full re-traversal works fine.

---

> **Next**: [07-future-mouse-inspector.md](./07-future-mouse-inspector.md) — Planning the mouse-based element inspector.
