/**
 * A single node in the macOS Accessibility tree.
 * 
 * This interface mirrors the Rust `AXNode` struct and Swift `AXNodeModel`.
 * All property names use camelCase (matching Rust's serde rename and Swift's Codable).
 */
export interface AXNode {
    /** Unique ID within a single traversal (e.g., "n_0", "n_1"). Not stable across refreshes. */
    id: string;

    /** Accessibility role: "AXButton", "AXWindow", "AXStaticText", etc. */
    role: string;

    /** Optional subrole: "AXCloseButton", "AXSearchField", etc. */
    subrole?: string;

    /** Element title (button text, window title, menu item label). */
    title?: string;

    /** Accessibility description for screen readers. */
    description?: string;

    /** Label value. */
    label?: string;

    /** Help/tooltip text. */
    help?: string;

    /** Stringified value (text content, checkbox state, slider value). */
    value?: string;

    /** Screen-coordinate frame (position + size). */
    frame?: Frame;

    /** Whether the element is enabled/interactive. */
    enabled?: boolean;

    /** Whether the element has keyboard focus. */
    focused?: boolean;

    /** Whether the element is selected (in lists/tables). */
    selected?: boolean;

    /** Actions this element supports: ["AXPress", "AXShowMenu"]. */
    actions: string[];

    /** All attribute names this element supports. */
    attributes: string[];

    /** Child nodes in the tree. */
    children: AXNode[];

    /** Number of direct children. */
    childCount: number;
}

/** Screen-coordinate frame of an element. */
export interface Frame {
    x: number;
    y: number;
    width: number;
    height: number;
}