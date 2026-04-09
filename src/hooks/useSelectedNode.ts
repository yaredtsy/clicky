import { useCallback, useEffect, useState } from "react";
import type { AXNode } from "../models";
import { clearHighlight, highlightElement } from "../services/accessibility";

interface UseSelectedNodeResult {
  selectedNode: AXNode | null;
  selectedId: string | null;
  select: (node: AXNode) => void;
  deselect: () => void;
}

export function useSelectedNode(): UseSelectedNodeResult {
  const [selectedNode, setSelectedNode] = useState<AXNode | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);

  useEffect(() => {
    void (async () => {
      try {
        if (selectedNode?.frame) {
          await highlightElement(selectedNode.frame, {
            title: selectedNode.title,
            description: selectedNode.description,
          });
        } else {
          await clearHighlight();
        }
      } catch {
        /* macOS-only overlay, or IPC unavailable */
      }
    })();
    return () => {
      void clearHighlight().catch(() => {});
    };
  }, [selectedNode]);

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
