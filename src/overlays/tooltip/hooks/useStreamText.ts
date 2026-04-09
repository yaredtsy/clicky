import { useEffect, useState } from "react";

/**
 * Reveals `text` one character at a time while `active` is true.
 * No caret, no CSS motion — only the string growing (ChatGPT-style stream).
 */
export function useStreamText(
  text: string,
  active: boolean,
  msPerChar: number,
): string {
  const [len, setLen] = useState(0);

  useEffect(() => {
    setLen(0);
  }, [text, active]);

  useEffect(() => {
    if (!active || len >= text.length) return;
    const id = window.setTimeout(() => setLen((n) => n + 1), msPerChar);
    return () => window.clearTimeout(id);
  }, [active, len, msPerChar, text.length]);

  if (!active) return "";
  return text.slice(0, len);
}
