// REF.2.P5 — Rank tooltip hover state (LAUNCH.1.E).
//
// `rect` is captured at the moment the hover delay fires (400 ms) so
// subsequent renders can read it without touching a ref. The timer ref is
// shared with the result-row mouse handlers — they call `start(index, rect)`
// on enter and `end()` on leave.

import { useRef, useState } from "react";

export interface RankHoverState {
  index: number;
  rect: DOMRect;
}

export interface UseRankHover {
  hover: RankHoverState | null;
  start: (index: number, rect: DOMRect) => void;
  end: () => void;
  hoverTimerRef: React.RefObject<ReturnType<typeof setTimeout> | null>;
}

export function useRankHover(): UseRankHover {
  const [hover, setHover] = useState<RankHoverState | null>(null);
  const hoverTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const start = (index: number, rect: DOMRect) => {
    if (hoverTimerRef.current) clearTimeout(hoverTimerRef.current);
    hoverTimerRef.current = setTimeout(() => {
      setHover({ index, rect });
    }, 400);
  };

  const end = () => {
    if (hoverTimerRef.current) {
      clearTimeout(hoverTimerRef.current);
      hoverTimerRef.current = null;
    }
    setHover(null);
  };

  return { hover, start, end, hoverTimerRef };
}
