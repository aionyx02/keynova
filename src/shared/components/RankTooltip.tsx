import { useI18n } from "../../i18n/useI18n";
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
 * Small fixed-position tooltip rendering "why this rank" for the
 * hovered result. Positioned to the right of the row by default; flips to the
 * left if it would overflow the viewport.
 */
export function RankTooltip({ breakdown, anchorRect, visible }: Props) {
  const t = useI18n().rank;
  if (!visible || !breakdown || !anchorRect) return null;

  const total =
    breakdown.base +
    breakdown.workspace_boost +
    breakdown.config_boost +
    breakdown.recency_boost +
    breakdown.frequency_boost +
    breakdown.noise_penalty;
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
        <span className="text-gray-500 uppercase tracking-wider text-[9px]">{t.title}</span>
        <span className="font-mono font-semibold text-gray-100">{total}</span>
      </div>
      <div className="grid grid-cols-[max-content_1fr] gap-x-2 gap-y-0.5 font-mono text-[10px]">
        <span className="text-gray-500">{t.base}</span>
        <span className="text-right text-gray-300">{breakdown.base}</span>
        {breakdown.workspace_boost !== 0 && (
          <>
            <span className="text-gray-500">{t.workspace}</span>
            <span className="text-right text-amber-300">{fmtSigned(breakdown.workspace_boost)}</span>
          </>
        )}
        {breakdown.config_boost !== 0 && (
          <>
            <span className="text-gray-500">{t.config}</span>
            <span className="text-right text-violet-300">{fmtSigned(breakdown.config_boost)}</span>
          </>
        )}
        <span className="text-gray-500">{t.recency}</span>
        <span className="text-right text-sky-300">{fmtSigned(breakdown.recency_boost)}</span>
        <span className="text-gray-500">{t.frequency}</span>
        <span className="text-right text-emerald-300">{fmtSigned(breakdown.frequency_boost)}</span>
        {breakdown.noise_penalty !== 0 && (
          <>
            <span className="text-gray-500">{t.noise}</span>
            <span className="text-right text-rose-300">{fmtSigned(breakdown.noise_penalty)}</span>
          </>
        )}
      </div>
    </div>
  );
}
