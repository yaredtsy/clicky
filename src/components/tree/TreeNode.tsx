import { useCallback, useState, type MouseEvent } from "react";
import type { AXNode } from "@/models";
import { cn } from "@/lib/utils";
import { getRoleIcon, getDisplayLabel } from "@/utils/tree-utils";
import { Badge } from "@/components/ui/badge";

interface TreeNodeProps {
  node: AXNode;
  depth: number;
  selectedId: string | null;
  onSelect: (node: AXNode) => void;
}

export function TreeNode({
  node,
  depth,
  selectedId,
  onSelect,
}: TreeNodeProps) {
  const [expanded, setExpanded] = useState(depth < 2);
  const hasChildren = node.children.length > 0;
  const isSelected = node.id === selectedId;

  const handleToggle = useCallback(
    (e: MouseEvent) => {
      e.stopPropagation();
      if (hasChildren) {
        setExpanded((prev) => !prev);
      }
    },
    [hasChildren],
  );

  const handleSelect = useCallback(
    (e: MouseEvent) => {
      e.stopPropagation();
      onSelect(node);
    },
    [node, onSelect],
  );

  const icon = getRoleIcon(node.role);
  const label = getDisplayLabel(node);

  return (
    <div className="min-w-0">
      <div
        className={cn(
          "flex cursor-pointer items-center gap-1 rounded-md py-0.5 pr-2 pl-1 font-mono text-[13px] leading-[22px] whitespace-nowrap select-none transition-colors",
          "hover:bg-muted/60",
          isSelected &&
            "bg-primary/15 ring-1 ring-primary/35 hover:bg-primary/20",
        )}
        style={{ paddingLeft: `${depth * 16 + 4}px` }}
        onClick={handleSelect}
        role="treeitem"
        aria-expanded={hasChildren ? expanded : undefined}
        aria-selected={isSelected}
        data-node-id={node.id}
      >
        <span
          className={cn(
            "w-3.5 shrink-0 text-center text-[10px] text-muted-foreground",
            hasChildren && "cursor-pointer hover:text-foreground",
          )}
          onClick={handleToggle}
        >
          {hasChildren ? (expanded ? "▼" : "▶") : "\u00a0"}
        </span>

        <span className="shrink-0 text-sm" title={node.role}>
          {icon}
        </span>

        <span className="shrink-0 font-medium text-sky-400 dark:text-sky-300">
          {node.role}
        </span>

        {label ? (
          <span
            className="max-w-[200px] shrink truncate text-violet-400 dark:text-violet-300"
            title={label}
          >
            &quot;{label}&quot;
          </span>
        ) : null}

        {hasChildren ? (
          <Badge
            variant="secondary"
            className="ml-auto h-5 shrink-0 px-1.5 text-[10px] font-normal"
          >
            {node.childCount}
          </Badge>
        ) : null}
      </div>

      {expanded && hasChildren ? (
        <div role="group">
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
      ) : null}
    </div>
  );
}
