// REF.6.B — Discovery hint for the prefix-keyword capability mode.
//
// Rendered above the empty palette state so users discover that typing
// `explain <q>` / `summarize <text>` (and future `cmd` / `fix` / `next`)
// switches the result area to a Gemini-style answer card. Hideable via
// `launcher.show_capability_hint = false`.

import type { ReactElement } from "react";

interface Props {
  visible: boolean;
}

interface PrefixHint {
  prefix: string;
  args: string;
}

// Order matches the docx Section 4 capabilities; only the first two are
// wired in REF.6.B. Listed prefixes that are not yet matched by
// `parseCapabilityPrefix` will fall through to normal search until the
// matching batch lands (REF.6.D / .E / .F).
const HINTS: ReadonlyArray<PrefixHint> = [
  { prefix: "explain", args: "<question>" },
  { prefix: "summarize", args: "<text>" },
  { prefix: "cmd", args: "<intent>" },
  { prefix: "fix", args: "<error>" },
  { prefix: "next", args: "" },
];

export function CapabilityHintLine({ visible }: Props): ReactElement | null {
  if (!visible) return null;
  return (
    <div className="kn-panel-shell overflow-hidden rounded-t-none border-t-0 bg-white/[0.015] px-4 py-2 text-[11px] text-[color:var(--kn-text-muted)]">
      <span className="text-[color:var(--kn-text-faint)]">Try: </span>
      {HINTS.map((h, i) => (
        <span key={h.prefix}>
          {i > 0 && <span className="mx-1.5 text-[color:var(--kn-text-faint)]">/</span>}
          <span className="font-medium text-[color:var(--kn-text-soft)]">{h.prefix}</span>
          {h.args && <span className="font-mono text-[color:var(--kn-text-faint)]"> {h.args}</span>}
        </span>
      ))}
    </div>
  );
}
