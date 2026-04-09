import type { AXNode } from "@/models";
import { cn } from "@/lib/utils";
import { TreeNode } from "./TreeNode";

interface TreeViewProps {
  root: AXNode;
  selectedId: string | null;
  onSelect: (node: AXNode) => void;
  className?: string;
}

export function TreeView({
  root,
  selectedId,
  onSelect,
  className,
}: TreeViewProps) {
  return (
    <div
      className={cn(
        "h-full overflow-auto bg-card/40 py-2 [scrollbar-width:thin]",
        className,
      )}
      role="tree"
      aria-label="Accessibility tree"
    >
      <TreeNode
        node={root}
        depth={0}
        selectedId={selectedId}
        onSelect={onSelect}
      />
    </div>
  );
}
