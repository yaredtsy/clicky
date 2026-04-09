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