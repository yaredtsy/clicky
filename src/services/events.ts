/**
 * Tauri event listener helpers.
 */
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

/** Listen for frontmost app changes (emitted by the monitor). */
export function onFrontmostChanged(
    callback: (bundleId: string) => void
): Promise<UnlistenFn> {
    return listen<string>('ax-frontmost-changed', (event) => {
        callback(event.payload);
    });
}