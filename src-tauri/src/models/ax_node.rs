use serde::{Deserialize, Serialize};

/// A single node in the macOS Accessibility tree.
///
/// This struct mirrors `AXNodeModel` in Swift and `AXNode` in TypeScript.
/// Serde handles the naming convention conversion:
/// - Rust: `child_count` (snake_case)
/// - JSON: `childCount` (camelCase)
/// - TypeScript: `childCount` (camelCase)
///
/// ## Why #[serde(rename_all = "camelCase")]?
///
/// JavaScript/TypeScript convention uses camelCase for object properties.
/// Rust convention uses snake_case. Rather than manually renaming each field,
/// serde does it automatically. This means the JSON produced by Swift's
/// `Codable` (which uses camelCase by default) matches this struct exactly.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AXNode {
    /// Unique ID within a single traversal (e.g., "n_0", "n_1").
    /// Not stable across traversals.
    pub id: String,

    /// Accessibility role: "AXButton", "AXWindow", "AXStaticText", etc.
    pub role: String,

    /// Optional subrole: "AXCloseButton", "AXSearchField", etc.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subrole: Option<String>,

    /// Element title (button text, window title, menu item label).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// Accessibility description for screen readers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Label value (less common, newer API).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,

    /// Help/tooltip text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,

    /// Stringified value (text content, checkbox state, slider value).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,

    /// Screen-coordinate frame (position + size).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame: Option<Frame>,

    /// Whether the element is enabled/interactive.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,

    /// Whether the element has keyboard focus.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focused: Option<bool>,

    /// Whether the element is selected (in lists/tables).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected: Option<bool>,

    /// Actions this element supports: ["AXPress", "AXShowMenu"].
    pub actions: Vec<String>,

    /// All attribute names this element supports.
    pub attributes: Vec<String>,

    /// Child nodes in the tree.
    pub children: Vec<AXNode>,

    /// Number of direct children.
    pub child_count: usize,
}

/// Screen-coordinate frame of an element.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frame {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Information about the inspected application.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub pid: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bundle_identifier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Top-level response containing app info + the full AX tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AXTreeResponse {
    pub app: AppInfo,
    pub root: AXNode,
    pub node_count: usize,
    pub truncated: bool,
}
