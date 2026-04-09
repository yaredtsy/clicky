import { createRoot } from "react-dom/client";
import { initTooltipBridge } from "./bridge";
import { TooltipOverlayApp } from "./TooltipOverlayApp";
import "./tooltip.css";

initTooltipBridge();

createRoot(document.getElementById("root")!).render(<TooltipOverlayApp />);
