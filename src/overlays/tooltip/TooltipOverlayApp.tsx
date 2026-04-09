import { useEffect, useState } from "react";
import { subscribeTooltipPayload } from "./bridge";
import { TooltipCard } from "./components/TooltipCard";
import type { TooltipOverlayPayload } from "./types";

export function TooltipOverlayApp() {
  const [payload, setPayload] = useState<TooltipOverlayPayload | null>(null);

  useEffect(() => {
    return subscribeTooltipPayload(setPayload);
  }, []);

  if (!payload) {
    return <div className="h-full w-full bg-transparent" />;
  }

  return <TooltipCard payload={payload} />;
}
