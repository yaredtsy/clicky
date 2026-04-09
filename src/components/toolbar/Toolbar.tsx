import { Loader2, RefreshCw, Smartphone, Square, Play } from "lucide-react";
import type { AppInfo } from "@/models";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";

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
  className?: string;
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
  className,
}: ToolbarProps) {
  const appLabel = appInfo
    ? `${appInfo.name ?? "Unknown"} (${appInfo.bundleIdentifier ?? `PID: ${appInfo.pid}`})`
    : "No app selected";

  return (
    <div
      className={cn(
        "flex flex-wrap items-center gap-3 border-b border-border bg-card/80 px-3 py-2",
        className,
      )}
    >
      <div className="flex min-w-0 flex-1 flex-wrap items-center gap-2">
        <Smartphone className="size-4 shrink-0 text-muted-foreground" />
        <span className="truncate text-sm text-foreground">{appLabel}</span>
        {nodeCount > 0 ? (
          <Badge variant="secondary" className="shrink-0 font-normal">
            {nodeCount} nodes
            {truncated ? " (truncated)" : ""}
          </Badge>
        ) : null}
      </div>

      <div className="flex shrink-0 flex-wrap gap-2">
        <Button
          variant="outline"
          size="sm"
          onClick={onRefresh}
          disabled={loading}
          title="Refresh tree"
        >
          {loading ? (
            <Loader2 className="size-3.5 animate-spin" />
          ) : (
            <RefreshCw className="size-3.5" />
          )}
          Refresh
        </Button>

        {monitoring ? (
          <Button
            variant="destructive"
            size="sm"
            onClick={onStopMonitoring}
            title="Stop monitoring"
          >
            <Square className="size-3.5" />
            Stop
          </Button>
        ) : (
          <Button
            variant="secondary"
            size="sm"
            onClick={onStartMonitoring}
            title="Auto-refresh when switching apps"
            className="border-emerald-500/30 text-emerald-600 dark:text-emerald-400"
          >
            <Play className="size-3.5" />
            Monitor
          </Button>
        )}
      </div>

      {error ? (
        <div
          className="w-full truncate rounded-md border border-destructive/25 bg-destructive/10 px-2 py-1.5 text-xs text-destructive"
          title={error}
        >
          {error}
        </div>
      ) : null}
    </div>
  );
}
