// Dev-only on-screen Ctrl+K -> input-ready timing badge.
//
// Renders the latest input-ready delta + median + sample count in a corner of
// the palette so the number is readable during dogfood without opening devtools
// (which would steal focus and trigger the launcher's hide-on-blur). Returns
// null in production: `timingEnabled()` is false there and the whole subtree is
// tree-shaken out by the `import.meta.env.DEV` guard inside it.

import { useEffect, useState } from "react";

import { perfStats, subscribePerf, timingEnabled } from "./devTiming";

export function PerfBadge(): React.ReactElement | null {
  const [, forceRender] = useState(0);

  useEffect(() => {
    if (!timingEnabled()) return;
    return subscribePerf(() => forceRender((n) => n + 1));
  }, []);

  if (!timingEnabled()) return null;

  const { last, median, n } = perfStats();

  return (
    <div
      className="pointer-events-none fixed bottom-1 right-2 z-50 select-none text-right font-mono text-[10px] leading-tight text-emerald-400/70"
      aria-hidden="true"
    >
      <div>input-ready</div>
      <div className="text-emerald-300/90">{last === null ? "—" : `${last.toFixed(1)}ms`}</div>
      <div>{n === 0 ? "no samples" : `med ${median.toFixed(1)} n=${n}`}</div>
    </div>
  );
}
