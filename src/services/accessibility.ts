/**
 * Tauri IPC wrappers for Accessibility commands.
 * 
 * This service encapsulates all `invoke()` calls. Components and hooks
 * should call these functions instead of calling `invoke()` directly.
 * 
 * Benefits:
 * - Single place to update if command names change
 * - Type-safe return values
 * - Consistent error handling
 */
import { invoke } from '@tauri-apps/api/core';
import type { AXTreeResponse, Frame } from '../models';

/** Check if Accessibility permission is granted (no prompt). */
export async function checkPermission(): Promise<boolean> {
    return invoke<boolean>('check_accessibility_permission');
}

/** Request Accessibility permission (may show system prompt). */
export async function requestPermission(): Promise<boolean> {
    return invoke<boolean>('request_accessibility_permission');
}

/** Get the full accessibility tree of the frontmost app. */
export async function getAccessibilityTree(): Promise<AXTreeResponse> {
    return invoke<AXTreeResponse>('get_accessibility_tree');
}

/** Start monitoring frontmost app changes. */
export async function startMonitor(): Promise<void> {
    return invoke<void>('start_accessibility_monitor');
}

/** Stop monitoring frontmost app changes. */
export async function stopMonitor(): Promise<void> {
    return invoke<void>('stop_accessibility_monitor');
}

/** Dump frontmost app AX tree to an XML file. */
export async function dumpToFile(path?: string): Promise<string> {
    return invoke<string>('dump_accessibility_tree_to_file', { path });
}

/** Show the highlight overlay at the given AX screen frame (top-left origin). */
export async function highlightElement(frame: Frame): Promise<void> {
    return invoke<void>('highlight_element', { frame });
}

/** Hide the highlight overlay. */
export async function clearHighlight(): Promise<void> {
    return invoke<void>('clear_highlight');
}