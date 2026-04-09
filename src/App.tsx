import { useEffect } from "react";
import { usePermission } from "@/hooks/usePermission";
import { useAccessibilityTree } from "@/hooks/useAccessibilityTree";
import { useSelectedNode } from "@/hooks/useSelectedNode";
import { PermissionGate } from "@/components/permission";
import { Toolbar } from "@/components/toolbar";
import { TreeView } from "@/components/tree";
import { DetailPanel } from "@/components/detail";
import { Separator } from "@/components/ui/separator";

function App() {
  const permission = usePermission();
  const axTree = useAccessibilityTree();
  const selection = useSelectedNode();

  useEffect(() => {
    selection.deselect();
  }, [axTree.tree, selection.deselect]);

  return (
    <PermissionGate
      status={permission.status}
      error={permission.error}
      onRequest={permission.request}
      onRefresh={permission.refresh}
    >
      <div className="flex h-screen min-h-0 flex-col bg-background">
        <header className="shrink-0 border-b border-border px-3 py-2">
          <h1 className="font-heading text-sm font-semibold tracking-tight">
            Claw Inspector
          </h1>
        </header>

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

        <div className="flex min-h-0 flex-1">
          <div className="flex min-h-0 min-w-0 flex-1 flex-col border-r border-border">
            <div className="shrink-0 px-3 py-1.5 text-xs font-medium text-muted-foreground">
              Tree
            </div>
            <Separator />
            <div className="min-h-0 flex-1">
              {axTree.tree ? (
                <TreeView
                  root={axTree.tree}
                  selectedId={selection.selectedId}
                  onSelect={selection.select}
                />
              ) : (
                <div className="flex h-full items-center justify-center p-6 text-center text-sm text-muted-foreground">
                  <p>
                    Click <strong className="text-foreground">Refresh</strong> or{" "}
                    <strong className="text-foreground">Monitor</strong> to load the
                    accessibility tree.
                  </p>
                </div>
              )}
            </div>
          </div>

          <div className="flex min-h-0 min-w-0 flex-1 flex-col">
            <div className="shrink-0 px-3 py-1.5 text-xs font-medium text-muted-foreground">
              Details
            </div>
            <Separator />
            <div className="min-h-0 flex-1">
              <DetailPanel node={selection.selectedNode} />
            </div>
          </div>
        </div>

        <footer className="flex shrink-0 flex-wrap items-center gap-3 border-t border-border bg-card/50 px-3 py-1.5 text-xs text-muted-foreground">
          <span>
            {axTree.nodeCount > 0
              ? `${axTree.nodeCount} nodes`
              : "No tree loaded"}
          </span>
          {axTree.appInfo ? <span>PID: {axTree.appInfo.pid}</span> : null}
          {axTree.monitoring ? (
            <span className="text-emerald-500 dark:text-emerald-400">
              ● Monitoring
            </span>
          ) : null}
        </footer>
      </div>
    </PermissionGate>
  );
}

export default App;
