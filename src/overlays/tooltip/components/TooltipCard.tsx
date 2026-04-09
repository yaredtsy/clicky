import type { TooltipOverlayPayload } from "../types";
import { useStreamText } from "../hooks/useStreamText";

/** Shown when AX title is missing — neutral filler copy. */
const PLACEHOLDER_TITLE = "Lorem ipsum";

/** Shown when AX description is missing. */
const PLACEHOLDER_DESCRIPTION =
  "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris.";

const TITLE_MS = 12;
const DESCRIPTION_MS = 5;

interface TooltipCardProps {
  payload: TooltipOverlayPayload;
}

export function TooltipCard({ payload }: TooltipCardProps) {
  const rawT = payload.title?.trim() ? payload.title : "";
  const rawD = payload.description?.trim() ? payload.description : "";
  const titleText = rawT || PLACEHOLDER_TITLE;
  const descText = rawD || PLACEHOLDER_DESCRIPTION;

  const titleVisible = useStreamText(titleText, true, TITLE_MS);
  const titleDone = titleVisible.length >= titleText.length;
  const descVisible = useStreamText(descText, titleDone, DESCRIPTION_MS);

  return (
    <div
      className="flex h-full flex-col gap-3 overflow-hidden rounded-2xl border-2 shadow bg-white px-3.5 py-3 text-black"
      data-placement={payload.placement || undefined}
      role="status"
      aria-live="polite"
    >
      <section>
        <div className="text-[10px] font-bold uppercase tracking-wider text-black">
          Title
        </div>
        <p className="mt-1 min-h-[1.35em] wrap-break-word text-sm font-medium leading-relaxed text-black">
          {titleVisible}
        </p>
      </section>
      <section>
        <div className="text-[10px] font-bold uppercase tracking-wider text-black">
          Description
        </div>
        <p className="mt-1 min-h-[1.35em] line-clamp-6 wrap-break-word text-sm leading-relaxed text-black">
          {descVisible}
        </p>
      </section>
    </div>
  );
}
