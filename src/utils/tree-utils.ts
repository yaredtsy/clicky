import type { AXNode } from "../models";

const ROLE_ICONS: Record<string, string> = {
  AXApplication: "🖥",
  AXWindow: "🪟",
  AXSheet: "📋",
  AXDialog: "💬",
  AXGroup: "📦",
  AXScrollArea: "📜",
  AXSplitGroup: "↔️",
  AXTabGroup: "📑",
  AXToolbar: "🔧",
  AXButton: "🔘",
  AXCheckBox: "☑️",
  AXRadioButton: "🔘",
  AXSlider: "🎚",
  AXTextField: "📝",
  AXTextArea: "📄",
  AXStaticText: "💬",
  AXLink: "🔗",
  AXImage: "🖼",
  AXTable: "📊",
  AXList: "📋",
  AXOutline: "🌲",
  AXRow: "➡️",
  AXCell: "▪️",
  AXMenuBar: "🍔",
  AXMenuBarItem: "📎",
  AXMenu: "📋",
  AXMenuItem: "▸",
  AXPopUpButton: "🔽",
  AXWebArea: "🌐",
  AXHeading: "📌",
  AXTruncated: "⚠️",
  AXCycleRef: "🔄",
  AXUnknown: "❓",
};

export function getRoleIcon(role: string): string {
  return ROLE_ICONS[role] ?? "•";
}

export function getDisplayLabel(node: AXNode): string | null {
  if (node.title) return node.title;
  if (node.description) return node.description;
  if (node.label) return node.label;
  if (node.value && node.value.length <= 50) return node.value;
  if (node.value) return `${node.value.slice(0, 47)}...`;
  return null;
}

export function flattenTree(node: AXNode): AXNode[] {
  const result: AXNode[] = [node];
  for (const child of node.children) {
    result.push(...flattenTree(child));
  }
  return result;
}

export function findNodeById(root: AXNode, id: string): AXNode | null {
  if (root.id === id) return root;
  for (const child of root.children) {
    const found = findNodeById(child, id);
    if (found) return found;
  }
  return null;
}

export function getNodePath(root: AXNode, targetId: string): AXNode[] {
  if (root.id === targetId) return [root];
  for (const child of root.children) {
    const path = getNodePath(child, targetId);
    if (path.length > 0) return [root, ...path];
  }
  return [];
}
