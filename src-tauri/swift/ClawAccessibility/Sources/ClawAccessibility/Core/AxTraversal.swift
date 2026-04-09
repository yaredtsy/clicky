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