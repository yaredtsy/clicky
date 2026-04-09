import type { AXNode } from "./ax-tree";

/** Information about the inspected application. */
export interface AppInfo {
    pid: number;
    bundleIdentifier?: string;
    name?: string;
}

/** Top-level response from the get_accessibility_tree command. */
export interface AXTreeResponse {
    app: AppInfo;
    root: AXNode;
    nodeCount: number;
    truncated: boolean;
}