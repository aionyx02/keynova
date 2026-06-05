// Discovery hint for the prefix-keyword capability mode.
//
// Rendered above the empty palette state so users discover that typing
// `explain <q>` / `summarize <text>` (and future `cmd` / `fix` / `next`)
// switches the result area to a Gemini-style answer card. Hideable via
// `launcher.show_capability_hint = false`.

import type { ReactElement } from "react";

import { useI18n } from "../../i18n/useI18n";

interface Props {
  visible: boolean;
}

interface PrefixHint {
  prefix: string;
}

// All listed prefixes are wired in `parseCapabilityPrefix`. `remember` / `recall`
// drive the local personal-memory loop.
const HINTS: ReadonlyArray<PrefixHint> = [
  { prefix: "explain" },
  { prefix: "summarize" },
  { prefix: "cmd" },
  { prefix: "fix" },
  { prefix: "remember" },
  { prefix: "recall" },
  { prefix: "next" },
];

export function CapabilityHintLine({ visible }: Props): ReactElement | null {
  const t = useI18n().capabilityHint;
  if (!visible) return null;
  return (
    <div className="kn-panel-shell overflow-hidden rounded-t-none border-t-0 bg-white/[0.015] px-4 py-2 text-[11px] text-[color:var(--kn-text-muted)]">
      <span className="text-[color:var(--kn-text-faint)]">{t.try}</span>
      {HINTS.map(({ prefix }, i) => {
        const args = t.args[prefix];
        return (
          <span key={prefix}>
            {i > 0 && <span className="mx-1.5 text-[color:var(--kn-text-faint)]">/</span>}
            <span className="font-medium text-[color:var(--kn-text-soft)]">{prefix}</span>
            {args && <span className="font-mono text-[color:var(--kn-text-faint)]"> {args}</span>}
          </span>
        );
      })}
    </div>
  );
}
