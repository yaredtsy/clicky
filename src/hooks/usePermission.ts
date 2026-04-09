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