# 05 — React Tree Viewer (Step-by-Step)

> **Goal**: Build an interactive tree viewer in React that displays the macOS AX tree as a folder/file hierarchy with a detail panel.

---

## Table of Contents

1. [What We're Building](#1-what-were-building)
2. [Step 1: Define TypeScript Models](#step-1-define-typescript-models)
3. [Step 2: Create Tauri IPC Service](#step-2-create-tauri-ipc-service)
4. [Step 3: Build the usePermission Hook](#step-3-build-the-usepermission-hook)
5. [Step 4: Build the useAccessibilityTree Hook](#step-4-build-the-useaccessibilitytree-hook)
6. [Step 5: Build the useSelectedNode Hook](#step-5-build-the-useselectednode-hook)
7. [Step 6: Build the TreeNode Component](#step-6-build-the-treenode-component)
8. [Step 7: Build the TreeView Component](#step-7-build-the-treeview-component)
9. [Step 8: Build the DetailPanel Component](#step-8-build-the-detailpanel-component)
10. [Step 9: Build the Toolbar Component](#step-9-build-the-toolbar-component)
11. [Step 10: Build the PermissionGate Component](#step-10-build-the-permissiongate-component)
12. [Step 11: Assemble the App](#step-11-assemble-the-app)
13. [Styling Strategy](#styling-strategy)
14. [Tree Utility Functions](#tree-utility-functions)

---

## 1. What We're Building

```
┌──────────────────────────────────────────────────────────────┐
│ ◉ Claw Inspector                                        □ ✕ │
├──────────────────────────────────────────────────────────────┤
│ 📱 Calculator (com.apple.calculator) │ 🔄 Refresh │ ⏸ Stop │
├─────────────────────────────┬────────────────────────────────┤
│ Tree                        │ Details                        │
│                             │                                │
│ ▼ 🖥 AXApplication          │ Role: AXButton                 │
│   ▼ 🪟 AXWindow             │ Subrole: —                     │
│     ▼ 📦 AXGroup            │ Title: "7"                     │
│       ▶ 🔘 AXButton "1"     │ Value: —                       │
│       ▶ 🔘 AXButton "2"     │ Frame: x=100 y=200 w=50 h=40  │
│       ▶ 🔘 AXButton "3"     │ Enabled: true                  │
│       █ 🔘 AXButton "7" ← █ │ Focused: false                 │
│       ▶ 🔘 AXButton "8"     │ Selected: false                │
│       ▶ 🔘 AXButton "9"     │                                │
│     ▶ 📦 AXGroup            │ Actions:                       │
│   ▶ 🍔 AXMenuBar            │  • AXPress                     │
│                             │                                │
│                             │ Attributes (17):               │
│                             │  AXRole, AXTitle, AXFrame,     │
│                             │  AXEnabled, AXFocused, ...     │
├─────────────────────────────┴────────────────────────────────┤
│ 47 nodes │ Calculator │ PID: 1234                            │
└──────────────────────────────────────────────────────────────┘
```

**Key interactions:**
- Click ▶/▼ to expand/collapse nodes
- Click a node to select it → detail panel shows its attributes
- Toolbar shows current app info and refresh/monitor controls
- Status bar shows node count

---

## Step 1: Define TypeScript Models

### `src/models/ax-tree.ts`

```typescript
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
```

### `src/models/app-info.ts`

```typescript
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
```

### `src/models/index.ts`

```typescript
export type { AXNode, Frame } from './ax-tree';
export type { AppInfo, AXTreeResponse } from './app-info';
```

---

## Step 2: Create Tauri IPC Service

### `src/services/accessibility.ts`

```typescript
/**
 * Tauri IPC wrappers for Accessibility commands.
 * 
 * This service encapsulates all `invoke()` calls. Components and hooks
 * should call these functions instead of calling `invoke()` directly.
 * 
 * Benefits:
 * - Single place to update if command names change
 * - Type-safe return values
 * - Consistent error handling
 */
import { invoke } from '@tauri-apps/api/core';
import type { AXTreeResponse } from '../models';

/** Check if Accessibility permission is granted (no prompt). */
export async function checkPermission(): Promise<boolean> {
  return invoke<boolean>('check_accessibility_permission');
}

/** Request Accessibility permission (may show system prompt). */
export async function requestPermission(): Promise<boolean> {
  return invoke<boolean>('request_accessibility_permission');
}

/** Get the full accessibility tree of the frontmost app. */
export async function getAccessibilityTree(): Promise<AXTreeResponse> {
  return invoke<AXTreeResponse>('get_accessibility_tree');
}

/** Start monitoring frontmost app changes. */
export async function startMonitor(): Promise<void> {
  return invoke<void>('start_accessibility_monitor');
}

/** Stop monitoring frontmost app changes. */
export async function stopMonitor(): Promise<void> {
  return invoke<void>('stop_accessibility_monitor');
}

/** Dump frontmost app AX tree to an XML file. */
export async function dumpToFile(path?: string): Promise<string> {
  return invoke<string>('dump_accessibility_tree_to_file', { path });
}
```

### `src/services/events.ts`

```typescript
/**
 * Tauri event listener helpers.
 */
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

/** Listen for frontmost app changes (emitted by the monitor). */
export function onFrontmostChanged(
  callback: (bundleId: string) => void
): Promise<UnlistenFn> {
  return listen<string>('ax-frontmost-changed', (event) => {
    callback(event.payload);
  });
}
```

---

## Step 3: Build the `usePermission` Hook

### `src/hooks/usePermission.ts`

```typescript
import { useCallback, useEffect, useState } from 'react';
import { checkPermission, requestPermission } from '../services/accessibility';

type PermissionStatus = 'loading' | 'allowed' | 'denied' | 'error';

interface UsePermissionResult {
  /** Current permission status. */
  status: PermissionStatus;
  /** Refresh the permission check (no prompt). */
  refresh: () => Promise<void>;
  /** Request permission (may show prompt). */
  request: () => Promise<void>;
  /** Error message if status is 'error'. */
  error: string | null;
}

/**
 * Hook to manage Accessibility permission state.
 * 
 * Checks permission on mount and provides functions to refresh and request.
 */
export function usePermission(): UsePermissionResult {
  const [status, setStatus] = useState<PermissionStatus>('loading');
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      const allowed = await checkPermission();
      setStatus(allowed ? 'allowed' : 'denied');
      setError(null);
    } catch (e) {
      setStatus('error');
      setError(String(e));
    }
  }, []);

  const request = useCallback(async () => {
    try {
      const alreadyTrusted = await requestPermission();
      setStatus(alreadyTrusted ? 'allowed' : 'denied');
      setError(null);
    } catch (e) {
      setStatus('error');
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  return { status, refresh, request, error };
}
```

---

## Step 4: Build the `useAccessibilityTree` Hook

### `src/hooks/useAccessibilityTree.ts`

```typescript
import { useCallback, useEffect, useState } from 'react';
import type { AXNode, AppInfo, AXTreeResponse } from '../models';
import { getAccessibilityTree, startMonitor, stopMonitor } from '../services/accessibility';
import { onFrontmostChanged } from '../services/events';

interface UseAccessibilityTreeResult {
  /** The root node of the AX tree. null if not loaded yet. */
  tree: AXNode | null;
  /** Info about the inspected app. */
  appInfo: AppInfo | null;
  /** Total nodes in the tree. */
  nodeCount: number;
  /** Whether the tree was truncated (hit depth/node limits). */
  truncated: boolean;
  /** Whether a tree fetch is in progress. */
  loading: boolean;
  /** Error message from the last failed operation. */
  error: string | null;
  /** Whether the monitor is running. */
  monitoring: boolean;
  /** Fetch the tree for the current frontmost app. */
  refresh: () => Promise<void>;
  /** Start monitoring frontmost app changes. */
  startMonitoring: () => Promise<void>;
  /** Stop monitoring. */
  stopMonitoring: () => Promise<void>;
}

/**
 * Central hook for managing the accessibility tree state.
 * 
 * Provides:
 * - Manual refresh of the current frontmost app's tree
 * - Automatic refresh via the frontmost monitor
 * - Loading/error state for UI feedback
 * 
 * ## Re-render Strategy
 * 
 * The tree object is replaced (not mutated) on each refresh. React's
 * reconciliation uses the `key={node.id}` pattern on TreeNode components
 * to efficiently update only changed subtrees.
 */
export function useAccessibilityTree(): UseAccessibilityTreeResult {
  const [tree, setTree] = useState<AXNode | null>(null);
  const [appInfo, setAppInfo] = useState<AppInfo | null>(null);
  const [nodeCount, setNodeCount] = useState(0);
  const [truncated, setTruncated] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [monitoring, setMonitoring] = useState(false);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const response: AXTreeResponse = await getAccessibilityTree();
      setTree(response.root);
      setAppInfo(response.app);
      setNodeCount(response.nodeCount);
      setTruncated(response.truncated);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  const startMonitoring = useCallback(async () => {
    try {
      await startMonitor();
      setMonitoring(true);
      setError(null);
      // Also fetch the initial tree
      await refresh();
    } catch (e) {
      setError(String(e));
    }
  }, [refresh]);

  const stopMonitoring = useCallback(async () => {
    try {
      await stopMonitor();
      setMonitoring(false);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  // Listen for frontmost app changes → auto-refresh
  useEffect(() => {
    if (!monitoring) return;

    let unlisten: (() => void) | undefined;
    
    void onFrontmostChanged((_bundleId) => {
      // When the frontmost app changes, re-fetch the tree
      void refresh();
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      unlisten?.();
    };
  }, [monitoring, refresh]);

  return {
    tree, appInfo, nodeCount, truncated,
    loading, error, monitoring,
    refresh, startMonitoring, stopMonitoring,
  };
}
```

---

## Step 5: Build the `useSelectedNode` Hook

### `src/hooks/useSelectedNode.ts`

```typescript
import { useCallback, useState } from 'react';
import type { AXNode } from '../models';

interface UseSelectedNodeResult {
  /** The currently selected node. */
  selectedNode: AXNode | null;
  /** The ID of the selected node (for highlight in tree). */
  selectedId: string | null;
  /** Select a node by clicking it in the tree. */
  select: (node: AXNode) => void;
  /** Clear selection. */
  deselect: () => void;
}

/**
 * Hook to manage the currently selected tree node.
 * 
 * Used to coordinate between TreeView (click to select)
 * and DetailPanel (show selected node's attributes).
 */
export function useSelectedNode(): UseSelectedNodeResult {
  const [selectedNode, setSelectedNode] = useState<AXNode | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);

  const select = useCallback((node: AXNode) => {
    setSelectedNode(node);
    setSelectedId(node.id);
  }, []);

  const deselect = useCallback(() => {
    setSelectedNode(null);
    setSelectedId(null);
  }, []);

  return { selectedNode, selectedId, select, deselect };
}
```

---

## Step 6: Build the TreeNode Component

### `src/components/tree/TreeNode.tsx`

```tsx
import { useState, useCallback, type MouseEvent } from 'react';
import type { AXNode } from '../../models';
import { getRoleIcon, getDisplayLabel } from '../../utils/tree-utils';
import './TreeNode.css';

interface TreeNodeProps {
  /** The AX node to render. */
  node: AXNode;
  /** Current nesting depth (for indentation). */
  depth: number;
  /** ID of the currently selected node. */
  selectedId: string | null;
  /** Callback when this node is clicked. */
  onSelect: (node: AXNode) => void;
}

/**
 * A single node in the tree view, with expand/collapse behavior.
 * 
 * ## Rendering Strategy
 * 
 * Children are rendered lazily: they are only mounted when the node
 * is expanded. This keeps initial render fast for large trees.
 * 
 * ## Why Not React.memo?
 * 
 * We intentionally DON'T memo this component because:
 * - `selectedId` changes on every selection, causing prop changes on every node
 * - The render is cheap (a few DOM elements)
 * - React's reconciliation with `key={node.id}` handles this efficiently
 * 
 * If profiling shows performance issues with 10K+ node trees, we'll add
 * virtualization (react-window) rather than memo.
 */
export function TreeNode({ node, depth, selectedId, onSelect }: TreeNodeProps) {
  const [expanded, setExpanded] = useState(depth < 2); // Auto-expand first 2 levels
  const hasChildren = node.children.length > 0;
  const isSelected = node.id === selectedId;

  const handleToggle = useCallback((e: MouseEvent) => {
    e.stopPropagation();
    if (hasChildren) {
      setExpanded((prev) => !prev);
    }
  }, [hasChildren]);

  const handleSelect = useCallback((e: MouseEvent) => {
    e.stopPropagation();
    onSelect(node);
  }, [node, onSelect]);

  const icon = getRoleIcon(node.role);
  const label = getDisplayLabel(node);

  return (
    <div className="tree-node-container">
      <div
        className={`tree-node ${isSelected ? 'tree-node--selected' : ''}`}
        style={{ paddingLeft: `${depth * 16 + 4}px` }}
        onClick={handleSelect}
        role="treeitem"
        aria-expanded={hasChildren ? expanded : undefined}
        aria-selected={isSelected}
        data-node-id={node.id}
      >
        {/* Expand/collapse toggle */}
        <span
          className={`tree-node__toggle ${hasChildren ? 'tree-node__toggle--has-children' : ''}`}
          onClick={handleToggle}
        >
          {hasChildren ? (expanded ? '▼' : '▶') : ' '}
        </span>

        {/* Role icon */}
        <span className="tree-node__icon" title={node.role}>
          {icon}
        </span>

        {/* Role name */}
        <span className="tree-node__role">{node.role}</span>

        {/* Label/title (if present) */}
        {label && (
          <span className="tree-node__label" title={label}>
            "{label}"
          </span>
        )}

        {/* Child count badge */}
        {hasChildren && (
          <span className="tree-node__badge">{node.childCount}</span>
        )}
      </div>

      {/* Children (rendered only when expanded) */}
      {expanded && hasChildren && (
        <div className="tree-node__children" role="group">
          {node.children.map((child) => (
            <TreeNode
              key={child.id}
              node={child}
              depth={depth + 1}
              selectedId={selectedId}
              onSelect={onSelect}
            />
          ))}
        </div>
      )}
    </div>
  );
}
```

### `src/components/tree/TreeNode.css`

```css
.tree-node-container {
  /* No extra styles — just a structural wrapper */
}

.tree-node {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 2px 8px 2px 4px;
  cursor: pointer;
  border-radius: 4px;
  font-size: 13px;
  font-family: 'SF Mono', 'Menlo', 'Consolas', monospace;
  line-height: 22px;
  white-space: nowrap;
  user-select: none;
  transition: background-color 0.1s ease;
}

.tree-node:hover {
  background-color: var(--color-hover, rgba(255, 255, 255, 0.06));
}

.tree-node--selected {
  background-color: var(--color-selected, rgba(59, 130, 246, 0.2));
  outline: 1px solid var(--color-selected-border, rgba(59, 130, 246, 0.4));
}

.tree-node--selected:hover {
  background-color: var(--color-selected-hover, rgba(59, 130, 246, 0.25));
}

.tree-node__toggle {
  width: 14px;
  text-align: center;
  font-size: 10px;
  color: var(--color-muted, #888);
  flex-shrink: 0;
}

.tree-node__toggle--has-children {
  cursor: pointer;
}

.tree-node__toggle--has-children:hover {
  color: var(--color-text, #eee);
}

.tree-node__icon {
  font-size: 14px;
  flex-shrink: 0;
}

.tree-node__role {
  color: var(--color-role, #7dd3fc);
  font-weight: 500;
}

.tree-node__label {
  color: var(--color-label, #a78bfa);
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 200px;
}

.tree-node__badge {
  background-color: var(--color-badge-bg, rgba(255, 255, 255, 0.1));
  color: var(--color-badge-text, #888);
  font-size: 10px;
  padding: 0 5px;
  border-radius: 8px;
  margin-left: auto;
  flex-shrink: 0;
}

.tree-node__children {
  /* Children inherit the container flow */
}
```

---

## Step 7: Build the TreeView Component

### `src/components/tree/TreeView.tsx`

```tsx
import type { AXNode } from '../../models';
import { TreeNode } from './TreeNode';
import './TreeView.css';

interface TreeViewProps {
  /** Root node of the tree. */
  root: AXNode;
  /** ID of the currently selected node. */
  selectedId: string | null;
  /** Called when a node is clicked. */
  onSelect: (node: AXNode) => void;
}

/**
 * Root container for the tree view.
 * 
 * This component provides:
 * - Scrollable container for the tree
 * - ARIA role="tree" for accessibility
 * - Visual styling for the tree panel
 */
export function TreeView({ root, selectedId, onSelect }: TreeViewProps) {
  return (
    <div className="tree-view" role="tree" aria-label="Accessibility tree">
      <TreeNode
        node={root}
        depth={0}
        selectedId={selectedId}
        onSelect={onSelect}
      />
    </div>
  );
}
```

### `src/components/tree/TreeView.css`

```css
.tree-view {
  overflow: auto;
  height: 100%;
  padding: 8px 0;
  background-color: var(--color-panel-bg, #1e1e2e);
}

/* Custom scrollbar for the tree */
.tree-view::-webkit-scrollbar {
  width: 8px;
  height: 8px;
}

.tree-view::-webkit-scrollbar-track {
  background: transparent;
}

.tree-view::-webkit-scrollbar-thumb {
  background-color: var(--color-scrollbar, rgba(255, 255, 255, 0.15));
  border-radius: 4px;
}

.tree-view::-webkit-scrollbar-thumb:hover {
  background-color: var(--color-scrollbar-hover, rgba(255, 255, 255, 0.25));
}
```

### `src/components/tree/index.ts`

```typescript
export { TreeView } from './TreeView';
export { TreeNode } from './TreeNode';
```

---

## Step 8: Build the DetailPanel Component

### `src/components/detail/DetailPanel.tsx`

```tsx
import type { AXNode } from '../../models';
import { getRoleIcon } from '../../utils/tree-utils';
import './DetailPanel.css';

interface DetailPanelProps {
  /** The currently selected node to display details for. */
  node: AXNode | null;
}

/**
 * Shows detailed attributes of the selected tree node.
 * 
 * Displays:
 * - All identity attributes (role, subrole, title, etc.)
 * - Frame position and size
 * - State flags (enabled, focused, selected)
 * - Available actions
 * - Full attribute name list
 */
export function DetailPanel({ node }: DetailPanelProps) {
  if (!node) {
    return (
      <div className="detail-panel detail-panel--empty">
        <p className="detail-panel__placeholder">
          Select a node in the tree to view its attributes
        </p>
      </div>
    );
  }

  return (
    <div className="detail-panel">
      {/* Header */}
      <div className="detail-panel__header">
        <span className="detail-panel__icon">{getRoleIcon(node.role)}</span>
        <span className="detail-panel__role">{node.role}</span>
        {node.subrole && (
          <span className="detail-panel__subrole">({node.subrole})</span>
        )}
      </div>

      {/* Identity Section */}
      <section className="detail-section">
        <h3 className="detail-section__title">Identity</h3>
        <table className="detail-table">
          <tbody>
            <DetailRow label="Role" value={node.role} />
            <DetailRow label="Subrole" value={node.subrole} />
            <DetailRow label="Title" value={node.title} />
            <DetailRow label="Description" value={node.description} />
            <DetailRow label="Label" value={node.label} />
            <DetailRow label="Help" value={node.help} />
            <DetailRow label="Value" value={node.value} />
            <DetailRow label="ID" value={node.id} muted />
          </tbody>
        </table>
      </section>

      {/* Frame Section */}
      {node.frame && (
        <section className="detail-section">
          <h3 className="detail-section__title">Frame</h3>
          <table className="detail-table">
            <tbody>
              <DetailRow label="X" value={`${node.frame.x.toFixed(1)}`} />
              <DetailRow label="Y" value={`${node.frame.y.toFixed(1)}`} />
              <DetailRow label="Width" value={`${node.frame.width.toFixed(1)}`} />
              <DetailRow label="Height" value={`${node.frame.height.toFixed(1)}`} />
            </tbody>
          </table>
        </section>
      )}

      {/* State Section */}
      <section className="detail-section">
        <h3 className="detail-section__title">State</h3>
        <table className="detail-table">
          <tbody>
            <DetailRow label="Enabled" value={formatBool(node.enabled)} />
            <DetailRow label="Focused" value={formatBool(node.focused)} />
            <DetailRow label="Selected" value={formatBool(node.selected)} />
            <DetailRow label="Children" value={`${node.childCount}`} />
          </tbody>
        </table>
      </section>

      {/* Actions Section */}
      {node.actions.length > 0 && (
        <section className="detail-section">
          <h3 className="detail-section__title">
            Actions ({node.actions.length})
          </h3>
          <ul className="detail-list">
            {node.actions.map((action) => (
              <li key={action} className="detail-list__item">
                {action}
              </li>
            ))}
          </ul>
        </section>
      )}

      {/* All Attributes Section */}
      {node.attributes.length > 0 && (
        <section className="detail-section">
          <h3 className="detail-section__title">
            All Attributes ({node.attributes.length})
          </h3>
          <div className="detail-tags">
            {node.attributes.map((attr) => (
              <span key={attr} className="detail-tag">
                {attr}
              </span>
            ))}
          </div>
        </section>
      )}
    </div>
  );
}

/** A single row in the detail table. */
function DetailRow({
  label,
  value,
  muted = false,
}: {
  label: string;
  value?: string | null;
  muted?: boolean;
}) {
  return (
    <tr className="detail-row">
      <td className="detail-row__label">{label}</td>
      <td className={`detail-row__value ${muted ? 'detail-row__value--muted' : ''}`}>
        {value ?? '—'}
      </td>
    </tr>
  );
}

function formatBool(val?: boolean | null): string {
  if (val === undefined || val === null) return '—';
  return val ? 'true' : 'false';
}
```

### `src/components/detail/DetailPanel.css`

```css
.detail-panel {
  padding: 16px;
  overflow: auto;
  height: 100%;
  background-color: var(--color-panel-bg, #1e1e2e);
}

.detail-panel--empty {
  display: flex;
  align-items: center;
  justify-content: center;
}

.detail-panel__placeholder {
  color: var(--color-muted, #888);
  font-size: 13px;
  text-align: center;
}

.detail-panel__header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding-bottom: 12px;
  border-bottom: 1px solid var(--color-border, rgba(255, 255, 255, 0.1));
  margin-bottom: 12px;
}

.detail-panel__icon {
  font-size: 20px;
}

.detail-panel__role {
  font-size: 16px;
  font-weight: 600;
  color: var(--color-role, #7dd3fc);
}

.detail-panel__subrole {
  font-size: 13px;
  color: var(--color-muted, #888);
}

/* Sections */
.detail-section {
  margin-bottom: 16px;
}

.detail-section__title {
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--color-muted, #888);
  margin: 0 0 8px 0;
}

/* Table */
.detail-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 13px;
}

.detail-row__label {
  color: var(--color-muted, #888);
  padding: 3px 12px 3px 0;
  white-space: nowrap;
  width: 80px;
  vertical-align: top;
}

.detail-row__value {
  color: var(--color-text, #e0e0e0);
  padding: 3px 0;
  word-break: break-all;
}

.detail-row__value--muted {
  color: var(--color-muted, #888);
  font-size: 11px;
  font-family: monospace;
}

/* Lists */
.detail-list {
  list-style: none;
  padding: 0;
  margin: 0;
}

.detail-list__item {
  font-size: 13px;
  padding: 2px 0;
  color: var(--color-text, #e0e0e0);
}

.detail-list__item::before {
  content: '•';
  color: var(--color-muted, #888);
  margin-right: 8px;
}

/* Tags */
.detail-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}

.detail-tag {
  font-size: 11px;
  padding: 2px 6px;
  border-radius: 4px;
  background-color: var(--color-badge-bg, rgba(255, 255, 255, 0.08));
  color: var(--color-muted, #aaa);
  font-family: monospace;
}
```

### `src/components/detail/index.ts`

```typescript
export { DetailPanel } from './DetailPanel';
```

---

## Step 9: Build the Toolbar Component

### `src/components/toolbar/Toolbar.tsx`

```tsx
import type { AppInfo } from '../../models';
import './Toolbar.css';

interface ToolbarProps {
  appInfo: AppInfo | null;
  nodeCount: number;
  truncated: boolean;
  loading: boolean;
  monitoring: boolean;
  error: string | null;
  onRefresh: () => void;
  onStartMonitoring: () => void;
  onStopMonitoring: () => void;
}

export function Toolbar({
  appInfo,
  nodeCount,
  truncated,
  loading,
  monitoring,
  error,
  onRefresh,
  onStartMonitoring,
  onStopMonitoring,
}: ToolbarProps) {
  const appLabel = appInfo
    ? `${appInfo.name ?? 'Unknown'} (${appInfo.bundleIdentifier ?? `PID: ${appInfo.pid}`})`
    : 'No app selected';

  return (
    <div className="toolbar">
      <div className="toolbar__app-info">
        <span className="toolbar__app-icon">📱</span>
        <span className="toolbar__app-label">{appLabel}</span>
        {nodeCount > 0 && (
          <span className="toolbar__node-count">
            {nodeCount} nodes{truncated ? ' (truncated)' : ''}
          </span>
        )}
      </div>

      <div className="toolbar__actions">
        <button
          className="toolbar__button"
          onClick={onRefresh}
          disabled={loading}
          title="Refresh tree"
        >
          {loading ? '⏳' : '🔄'} Refresh
        </button>

        {monitoring ? (
          <button
            className="toolbar__button toolbar__button--stop"
            onClick={onStopMonitoring}
            title="Stop monitoring"
          >
            ⏹ Stop
          </button>
        ) : (
          <button
            className="toolbar__button toolbar__button--start"
            onClick={onStartMonitoring}
            title="Auto-refresh when switching apps"
          >
            ▶ Monitor
          </button>
        )}
      </div>

      {error && (
        <div className="toolbar__error" title={error}>
          ⚠️ {error}
        </div>
      )}
    </div>
  );
}
```

### `src/components/toolbar/Toolbar.css`

```css
.toolbar {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 8px 12px;
  background-color: var(--color-toolbar-bg, #181825);
  border-bottom: 1px solid var(--color-border, rgba(255, 255, 255, 0.1));
  font-size: 13px;
  flex-wrap: wrap;
}

.toolbar__app-info {
  display: flex;
  align-items: center;
  gap: 6px;
  flex: 1;
  min-width: 0;
}

.toolbar__app-icon {
  font-size: 16px;
  flex-shrink: 0;
}

.toolbar__app-label {
  color: var(--color-text, #e0e0e0);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.toolbar__node-count {
  color: var(--color-muted, #888);
  font-size: 11px;
  flex-shrink: 0;
}

.toolbar__actions {
  display: flex;
  gap: 6px;
  flex-shrink: 0;
}

.toolbar__button {
  padding: 4px 10px;
  border: 1px solid var(--color-border, rgba(255, 255, 255, 0.15));
  border-radius: 6px;
  background: var(--color-button-bg, rgba(255, 255, 255, 0.06));
  color: var(--color-text, #e0e0e0);
  font-size: 12px;
  cursor: pointer;
  transition: all 0.15s ease;
  white-space: nowrap;
}

.toolbar__button:hover:not(:disabled) {
  background: var(--color-button-hover, rgba(255, 255, 255, 0.1));
}

.toolbar__button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.toolbar__button--start {
  border-color: rgba(34, 197, 94, 0.3);
  color: #22c55e;
}

.toolbar__button--stop {
  border-color: rgba(239, 68, 68, 0.3);
  color: #ef4444;
}

.toolbar__error {
  width: 100%;
  padding: 4px 8px;
  background: rgba(239, 68, 68, 0.1);
  border: 1px solid rgba(239, 68, 68, 0.2);
  border-radius: 4px;
  color: #fca5a5;
  font-size: 12px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
```

### `src/components/toolbar/index.ts`

```typescript
export { Toolbar } from './Toolbar';
```

---

## Step 10: Build the PermissionGate Component

### `src/components/permission/PermissionGate.tsx`

```tsx
import type { ReactNode } from 'react';
import './PermissionGate.css';

interface PermissionGateProps {
  status: 'loading' | 'allowed' | 'denied' | 'error';
  error: string | null;
  onRequest: () => void;
  onRefresh: () => void;
  children: ReactNode;
}

/**
 * Blocks rendering of children until Accessibility permission is granted.
 * 
 * Shows a permission request UI when permission is denied.
 */
export function PermissionGate({
  status,
  error,
  onRequest,
  onRefresh,
  children,
}: PermissionGateProps) {
  if (status === 'allowed') {
    return <>{children}</>;
  }

  return (
    <div className="permission-gate">
      <div className="permission-gate__card">
        <div className="permission-gate__icon">🔐</div>
        <h2 className="permission-gate__title">Accessibility Permission Required</h2>
        
        {status === 'loading' && (
          <p className="permission-gate__message">Checking permission status…</p>
        )}

        {status === 'denied' && (
          <>
            <p className="permission-gate__message">
              Claw Inspector needs Accessibility access to read other apps' UI structure.
            </p>
            <ol className="permission-gate__steps">
              <li>Click <strong>Request Permission</strong> below</li>
              <li>In System Settings, enable <strong>claw-kernel</strong></li>
              <li>Click <strong>Refresh Status</strong> to continue</li>
            </ol>
            <div className="permission-gate__actions">
              <button className="permission-gate__button--primary" onClick={onRequest}>
                Request Permission
              </button>
              <button className="permission-gate__button--secondary" onClick={onRefresh}>
                Refresh Status
              </button>
            </div>
          </>
        )}

        {status === 'error' && (
          <>
            <p className="permission-gate__error">{error}</p>
            <button className="permission-gate__button--secondary" onClick={onRefresh}>
              Retry
            </button>
          </>
        )}
      </div>
    </div>
  );
}
```

### `src/components/permission/PermissionGate.css`

```css
.permission-gate {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100vh;
  background-color: var(--color-bg, #11111b);
}

.permission-gate__card {
  max-width: 420px;
  padding: 32px;
  border-radius: 12px;
  background: var(--color-panel-bg, #1e1e2e);
  border: 1px solid var(--color-border, rgba(255, 255, 255, 0.1));
  text-align: center;
}

.permission-gate__icon {
  font-size: 48px;
  margin-bottom: 16px;
}

.permission-gate__title {
  font-size: 18px;
  font-weight: 600;
  color: var(--color-text, #e0e0e0);
  margin: 0 0 12px 0;
}

.permission-gate__message {
  color: var(--color-muted, #888);
  font-size: 14px;
  line-height: 1.5;
  margin: 0 0 16px 0;
}

.permission-gate__steps {
  text-align: left;
  color: var(--color-muted, #aaa);
  font-size: 13px;
  line-height: 1.8;
  padding-left: 20px;
  margin: 0 0 20px 0;
}

.permission-gate__actions {
  display: flex;
  gap: 8px;
  justify-content: center;
}

.permission-gate__button--primary {
  padding: 8px 20px;
  border: none;
  border-radius: 8px;
  background: #3b82f6;
  color: white;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: background 0.15s ease;
}

.permission-gate__button--primary:hover {
  background: #2563eb;
}

.permission-gate__button--secondary {
  padding: 8px 20px;
  border: 1px solid var(--color-border, rgba(255, 255, 255, 0.15));
  border-radius: 8px;
  background: transparent;
  color: var(--color-text, #e0e0e0);
  font-size: 14px;
  cursor: pointer;
  transition: background 0.15s ease;
}

.permission-gate__button--secondary:hover {
  background: rgba(255, 255, 255, 0.06);
}

.permission-gate__error {
  color: #fca5a5;
  font-size: 13px;
  margin: 0 0 16px 0;
}
```

### `src/components/permission/index.ts`

```typescript
export { PermissionGate } from './PermissionGate';
```

---

## Step 11: Assemble the App

### `src/App.tsx`

```tsx
import { usePermission } from './hooks/usePermission';
import { useAccessibilityTree } from './hooks/useAccessibilityTree';
import { useSelectedNode } from './hooks/useSelectedNode';
import { PermissionGate } from './components/permission';
import { Toolbar } from './components/toolbar';
import { TreeView } from './components/tree';
import { DetailPanel } from './components/detail';
import './App.css';

function App() {
  const permission = usePermission();
  const axTree = useAccessibilityTree();
  const selection = useSelectedNode();

  return (
    <PermissionGate
      status={permission.status}
      error={permission.error}
      onRequest={permission.request}
      onRefresh={permission.refresh}
    >
      <div className="app-layout">
        <Toolbar
          appInfo={axTree.appInfo}
          nodeCount={axTree.nodeCount}
          truncated={axTree.truncated}
          loading={axTree.loading}
          monitoring={axTree.monitoring}
          error={axTree.error}
          onRefresh={axTree.refresh}
          onStartMonitoring={axTree.startMonitoring}
          onStopMonitoring={axTree.stopMonitoring}
        />

        <div className="app-layout__content">
          <div className="app-layout__tree-panel">
            {axTree.tree ? (
              <TreeView
                root={axTree.tree}
                selectedId={selection.selectedId}
                onSelect={selection.select}
              />
            ) : (
              <div className="app-layout__empty">
                <p>Click <strong>Refresh</strong> or <strong>Monitor</strong> to load the accessibility tree.</p>
              </div>
            )}
          </div>

          <div className="app-layout__detail-panel">
            <DetailPanel node={selection.selectedNode} />
          </div>
        </div>

        {/* Status bar */}
        <div className="app-layout__status-bar">
          <span>
            {axTree.nodeCount > 0
              ? `${axTree.nodeCount} nodes`
              : 'No tree loaded'}
          </span>
          {axTree.appInfo && (
            <span>PID: {axTree.appInfo.pid}</span>
          )}
          {axTree.monitoring && (
            <span className="status-bar__monitoring">● Monitoring</span>
          )}
        </div>
      </div>
    </PermissionGate>
  );
}

export default App;
```

---

## Styling Strategy

### CSS Custom Properties (Design Tokens)

All colors are defined as CSS custom properties in `App.css`. Components reference these tokens, not hardcoded colors:

```css
/* App.css — add these at the top */
:root {
  /* Catppuccin Mocha-inspired dark theme */
  --color-bg: #11111b;
  --color-panel-bg: #1e1e2e;
  --color-toolbar-bg: #181825;
  --color-text: #cdd6f4;
  --color-muted: #6c7086;
  --color-border: rgba(255, 255, 255, 0.08);
  --color-hover: rgba(255, 255, 255, 0.05);
  --color-selected: rgba(137, 180, 250, 0.15);
  --color-selected-border: rgba(137, 180, 250, 0.3);
  --color-selected-hover: rgba(137, 180, 250, 0.2);
  --color-role: #89dceb;
  --color-label: #cba6f7;
  --color-badge-bg: rgba(255, 255, 255, 0.06);
  --color-badge-text: #6c7086;
  --color-scrollbar: rgba(255, 255, 255, 0.1);
  --color-scrollbar-hover: rgba(255, 255, 255, 0.2);
  --color-button-bg: rgba(255, 255, 255, 0.05);
  --color-button-hover: rgba(255, 255, 255, 0.08);
}
```

---

## Tree Utility Functions

### `src/utils/tree-utils.ts`

```typescript
import type { AXNode } from '../models';

/**
 * Map AX roles to emoji icons for the tree view.
 * 
 * Using emoji instead of an icon library keeps the bundle small
 * and works across all platforms without additional dependencies.
 */
const ROLE_ICONS: Record<string, string> = {
  AXApplication: '🖥',
  AXWindow: '🪟',
  AXSheet: '📋',
  AXDialog: '💬',
  AXGroup: '📦',
  AXScrollArea: '📜',
  AXSplitGroup: '↔️',
  AXTabGroup: '📑',
  AXToolbar: '🔧',
  AXButton: '🔘',
  AXCheckBox: '☑️',
  AXRadioButton: '🔘',
  AXSlider: '🎚',
  AXTextField: '📝',
  AXTextArea: '📄',
  AXStaticText: '💬',
  AXLink: '🔗',
  AXImage: '🖼',
  AXTable: '📊',
  AXList: '📋',
  AXOutline: '🌲',
  AXRow: '➡️',
  AXCell: '▪️',
  AXMenuBar: '🍔',
  AXMenuBarItem: '📎',
  AXMenu: '📋',
  AXMenuItem: '▸',
  AXPopUpButton: '🔽',
  AXWebArea: '🌐',
  AXHeading: '📌',
  AXTruncated: '⚠️',
  AXCycleRef: '🔄',
  AXUnknown: '❓',
};

/** Get an emoji icon for a given AX role. */
export function getRoleIcon(role: string): string {
  return ROLE_ICONS[role] ?? '•';
}

/**
 * Get the best display label for a node.
 * 
 * Priority: title > description > label > value (truncated)
 */
export function getDisplayLabel(node: AXNode): string | null {
  if (node.title) return node.title;
  if (node.description) return node.description;
  if (node.label) return node.label;
  if (node.value && node.value.length <= 50) return node.value;
  if (node.value) return node.value.slice(0, 47) + '...';
  return null;
}

/**
 * Flatten a tree into a list of all nodes (DFS order).
 * Useful for search, filtering, and statistics.
 */
export function flattenTree(node: AXNode): AXNode[] {
  const result: AXNode[] = [node];
  for (const child of node.children) {
    result.push(...flattenTree(child));
  }
  return result;
}

/**
 * Find a node by ID in the tree.
 */
export function findNodeById(root: AXNode, id: string): AXNode | null {
  if (root.id === id) return root;
  for (const child of root.children) {
    const found = findNodeById(child, id);
    if (found) return found;
  }
  return null;
}

/**
 * Get the path from root to a target node (for breadcrumb display).
 */
export function getNodePath(root: AXNode, targetId: string): AXNode[] {
  if (root.id === targetId) return [root];
  for (const child of root.children) {
    const path = getNodePath(child, targetId);
    if (path.length > 0) return [root, ...path];
  }
  return [];
}
```

### `src/utils/format.ts`

```typescript
/** Format a frame as a human-readable string. */
export function formatFrame(frame: { x: number; y: number; width: number; height: number }): string {
  return `(${frame.x.toFixed(0)}, ${frame.y.toFixed(0)}) ${frame.width.toFixed(0)} × ${frame.height.toFixed(0)}`;
}
```

---

> **Next**: [06-realtime-tree-sync.md](./06-realtime-tree-sync.md) — Making the tree update in real-time as the user switches apps.
