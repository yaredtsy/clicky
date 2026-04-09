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