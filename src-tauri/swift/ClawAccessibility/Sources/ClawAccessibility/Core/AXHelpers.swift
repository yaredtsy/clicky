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