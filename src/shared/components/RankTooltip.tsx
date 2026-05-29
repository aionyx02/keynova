import type { ScoreBreakdown } from "../../types/search";

interface Props {
  breakdown: ScoreBreakdown | undefined;
  anchorRect: DOMRect | null;
  visible: boolean;
}

function fmtSigned(n: number): string {
  return n >= 0 ? `+${n}` : `${n}`;
}

/**
 * LAUNCH.1.E — small fixed-position tooltip rendering "why this rank" for the
 * hovered result. Positioned to the right of the row by default; flips to the
 * left if it would overflow the viewport.
 */
export function RankTooltip({ breakdown, anchorRect, visible }: Props) {
  if (!visible || !breakdown || !anchorRect) return null;

  const total = breakdown.base + breakdown.recency_boost + breakdown.frequency_boost;
  const margin = 8;
  const estWidth = 220;
  const wantsRight = anchorRect.right + estWidth + margin <= window.innerWidth;
  const top = anchorRect.top + 4;
  const left = wantsRight
    ? anchorRect.right + margin
    : Math.max(margin, anchorRect.left - estWidth - margin);

  return (
    <div
      className="pointer-events-none fixed z-30 w-[220px] rounded-md border border-gray-700/60 bg-gray-950/95 px-3 py-2 text-[11px] text-gray-200 shadow-xl"
      style={{ top, left }}
      role="tooltip"
    >
      <div className="mb-1 flex items-baseline justify-between">
        <span className="text-gray-500 uppercase tracking-wider text-[9px]">Why this rank</span>
        <span className="font-mono font-semibold text-gray-100">{total}</span>
      </div>
      <div className="grid grid-cols-[max-content_1fr] gap-x-2 gap-y-0.5 font-mono text-[10px]">
        <span className="text-gray-500">base</span>
        <span className="text-right text-gray-300">{breakdown.base}</span>
        <span className="text-gray-500">recency</span>
        <span className="text-right text-sky-300">{fmtSigned(breakdown.recency_boost)}</span>
        <span className="text-gray-500">frequency</span>
        <span className="text-right text-emerald-300">{fmtSigned(breakdown.frequency_boost)}</span>
      </div>
    </div>
  );
}
