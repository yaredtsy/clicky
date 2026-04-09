import type { ReactNode } from "react";
import type { AXNode } from "@/models";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import { getRoleIcon } from "@/utils/tree-utils";
import { cn } from "@/lib/utils";

interface DetailPanelProps {
  node: AXNode | null;
  className?: string;
}

export function DetailPanel({ node, className }: DetailPanelProps) {
  if (!node) {
    return (
      <div
        className={cn(
          "flex h-full items-center justify-center bg-card/40 p-4",
          className,
        )}
      >
        <p className="text-center text-sm text-muted-foreground">
          Select a node in the tree to view its attributes
        </p>
      </div>
    );
  }

  return (
    <div
      className={cn(
        "h-full overflow-auto bg-card/40 p-4 text-sm",
        className,
      )}
    >
      <div className="mb-3 flex flex-wrap items-center gap-2 border-b border-border pb-3">
        <span className="text-xl">{getRoleIcon(node.role)}</span>
        <span className="text-base font-semibold text-sky-400 dark:text-sky-300">
          {node.role}
        </span>
        {node.subrole ? (
          <span className="text-muted-foreground">({node.subrole})</span>
        ) : null}
      </div>

      <section className="mb-4">
        <h3 className="mb-2 text-[11px] font-semibold tracking-wide text-muted-foreground uppercase">
          Identity
        </h3>
        <DetailTable>
          <DetailRow label="Role" value={node.role} />
          <DetailRow label="Subrole" value={node.subrole} />
          <DetailRow label="Title" value={node.title} />
          <DetailRow label="Description" value={node.description} />
          <DetailRow label="Label" value={node.label} />
          <DetailRow label="Help" value={node.help} />
          <DetailRow label="Value" value={node.value} />
          <DetailRow label="ID" value={node.id} muted />
        </DetailTable>
      </section>

      {node.frame ? (
        <>
          <Separator className="my-4" />
          <section className="mb-4">
            <h3 className="mb-2 text-[11px] font-semibold tracking-wide text-muted-foreground uppercase">
              Frame
            </h3>
            <DetailTable>
              <DetailRow label="X" value={`${node.frame.x.toFixed(1)}`} />
              <DetailRow label="Y" value={`${node.frame.y.toFixed(1)}`} />
              <DetailRow
                label="Width"
                value={`${node.frame.width.toFixed(1)}`}
              />
              <DetailRow
                label="Height"
                value={`${node.frame.height.toFixed(1)}`}
              />
            </DetailTable>
          </section>
        </>
      ) : null}

      <Separator className="my-4" />
      <section className="mb-4">
        <h3 className="mb-2 text-[11px] font-semibold tracking-wide text-muted-foreground uppercase">
          State
        </h3>
        <DetailTable>
          <DetailRow label="Enabled" value={formatBool(node.enabled)} />
          <DetailRow label="Focused" value={formatBool(node.focused)} />
          <DetailRow label="Selected" value={formatBool(node.selected)} />
          <DetailRow label="Children" value={`${node.childCount}`} />
        </DetailTable>
      </section>

      {node.actions.length > 0 ? (
        <>
          <Separator className="my-4" />
          <section className="mb-4">
            <h3 className="mb-2 text-[11px] font-semibold tracking-wide text-muted-foreground uppercase">
              Actions ({node.actions.length})
            </h3>
            <ul className="list-none space-y-1 p-0">
              {node.actions.map((action) => (
                <li key={action} className="flex gap-2 text-foreground">
                  <span className="text-muted-foreground">•</span>
                  <span>{action}</span>
                </li>
              ))}
            </ul>
          </section>
        </>
      ) : null}

      {node.attributes.length > 0 ? (
        <>
          <Separator className="my-4" />
          <section>
            <h3 className="mb-2 text-[11px] font-semibold tracking-wide text-muted-foreground uppercase">
              All attributes ({node.attributes.length})
            </h3>
            <div className="flex flex-wrap gap-1.5">
              {node.attributes.map((attr) => (
                <Badge
                  key={attr}
                  variant="outline"
                  className="font-mono text-[11px] font-normal"
                >
                  {attr}
                </Badge>
              ))}
            </div>
          </section>
        </>
      ) : null}
    </div>
  );
}

function DetailTable({ children }: { children: ReactNode }) {
  return (
    <table className="w-full border-collapse text-[13px]">
      <tbody>{children}</tbody>
    </table>
  );
}

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
    <tr>
      <td className="w-20 shrink-0 py-0.5 pr-3 align-top whitespace-nowrap text-muted-foreground">
        {label}
      </td>
      <td
        className={cn(
          "break-all py-0.5 text-foreground",
          muted && "font-mono text-[11px] text-muted-foreground",
        )}
      >
        {value ?? "—"}
      </td>
    </tr>
  );
}

function formatBool(val?: boolean | null): string {
  if (val === undefined || val === null) return "—";
  return val ? "true" : "false";
}
