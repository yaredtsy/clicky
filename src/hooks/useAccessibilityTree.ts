import { useCallback, useEffect, useState } from 'react';
import type { AXNode, AppInfo, AXTreeResponse } from '../models';
import { clearHighlight, getAccessibilityTree, startMonitor, stopMonitor } from '../services/accessibility';
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
        try {
            await clearHighlight();
        } catch {
            /* overlay only on macOS */
        }
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