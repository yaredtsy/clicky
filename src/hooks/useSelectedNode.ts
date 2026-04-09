import { useCallback, useState } from "react";
import type { AXNode } from "../models";

interface UseSelectedNodeResult {
  selectedNode: AXNode | null;
  selectedId: string | null;
  select: (node: AXNode) => void;
  deselect: () => void;
}

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
