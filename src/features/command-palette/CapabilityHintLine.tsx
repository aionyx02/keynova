// Discovery hint for the prefix-keyword capability mode.
//
// Rendered above the empty palette state so users discover that typing
// `explain <q>` / `summarize <text>` (and future `cmd` / `fix` / `next`)
// switches the result area to a Gemini-style answer card. Hideable via
// `launcher.show_capability_hint = false`.

import type { ReactElement } from "react";

import { UiIcon, type UiIconName } from "../../components/icons/UiIcon";
import { useI18n } from "../../i18n/useI18n";

interface Props {
  visible: boolean;
  onPickPrefix?: (query: string) => void;
}

interface PrefixHint {
  prefix: string;
  icon: UiIconName;
  query: string;
}

// All listed prefixes are wired in `parseCapabilityPrefix`. `remember` / `recall`
// drive the local personal-memory loop.
const HINTS: ReadonlyArray<PrefixHint> = [
  { prefix: "next", icon: "arrow-right", query: "next" },
  { prefix: "cmd", icon: "command", query: "cmd " },
  { prefix: "remember", icon: "database", query: "remember " },
  { prefix: "recall", icon: "search", query: "recall " },
  { prefix: "fix", icon: "settings", query: "fix " },
  { prefix: "explain", icon: "note", query: "explain " },
  { prefix: "summarize", icon: "file", query: "summarize " },
];

export function CapabilityHintLine({ visible, onPickPrefix }: Props): ReactElement | null {
  const t = useI18n().capabilityHint;
  if (!visible) return null;
  return (
    <div className="kn-panel-shell overflow-hidden rounded-t-none border-t-0 px-3 py-3">
      <div className="mb-2 flex items-center justify-between gap-3 px-1">
        <div className="min-w-0">
          <div className="text-[11px] font-semibold uppercase tracking-[0.12em] text-[color:var(--kn-text-faint)]">
            {t.title}
          </div>
          <div className="mt-0.5 truncate text-[11px] text-[color:var(--kn-text-muted)]">
            {t.subtitle}
          </div>
        </div>
        <span className="hidden shrink-0 text-[10px] text-[color:var(--kn-text-faint)] sm:inline">
          {t.try}
        </span>
      </div>

      <div className="kn-scroll flex gap-2 overflow-x-auto pb-0.5">
        {HINTS.map(({ prefix, icon, query }) => {
          const args = t.args[prefix];
          const label = t.labels[prefix] ?? prefix;
          const description = t.descriptions[prefix] ?? "";
          return (
            <button
              key={prefix}
              type="button"
              onClick={() => onPickPrefix?.(query)}
              className="kn-quick-start shrink-0"
            >
              <span className="flex h-8 w-8 shrink-0 items-center justify-center rounded-[8px] border border-white/10 bg-white/[0.035] text-[color:var(--kn-accent)]">
                <UiIcon name={icon} className="h-4 w-4" />
              </span>
              <span className="min-w-0 text-left">
                <span className="block text-xs font-semibold text-[color:var(--kn-text-soft)]">
                  {label}
                </span>
                <span className="block truncate font-mono text-[10px] text-[color:var(--kn-text-faint)]">
                  {prefix}
                  {args && ` ${args}`}
                </span>
              </span>
              {description && <span className="sr-only">{description}</span>}
            </button>
          );
        })}
      </div>
    </div>
  );
}
