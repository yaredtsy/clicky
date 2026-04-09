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