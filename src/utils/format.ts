/** Format a frame as a human-readable string. */
export function formatFrame(frame: {
  x: number;
  y: number;
  width: number;
  height: number;
}): string {
  return `(${frame.x.toFixed(0)}, ${frame.y.toFixed(0)}) ${frame.width.toFixed(0)} × ${frame.height.toFixed(0)}`;
}
