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