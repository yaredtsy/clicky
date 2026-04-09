import type { TooltipOverlayPayload } from "./types";

type Listener = (payload: TooltipOverlayPayload) => void;

let listener: Listener | null = null;
let pending: TooltipOverlayPayload | null = null;

/** React root calls this to receive payloads (handles early Rust eval before mount). */
export function subscribeTooltipPayload(fn: Listener): () => void {
  listener = fn;
  if (pending) {
    fn(pending);
    pending = null;
  }
  return () => {
    listener = null;
  };
}

/** Install global hook used by Rust `window.__applyTooltip({...})`. */
export function initTooltipBridge(): void {
  window.__applyTooltip = (d: TooltipOverlayPayload) => {
    if (listener) listener(d);
    else pending = d;
  };
}

declare global {
  interface Window {
    __applyTooltip?: (d: TooltipOverlayPayload) => void;
  }
}
