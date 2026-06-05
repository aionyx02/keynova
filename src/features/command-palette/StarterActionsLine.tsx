import type { ReactElement } from "react";

import { UiIcon, type UiIconName } from "../../components/icons/UiIcon";
import { useI18n } from "../../i18n/useI18n";

interface Props {
  visible: boolean;
  onPickQuery: (query: string) => void;
}

interface StarterSpec {
  key: string;
  icon: UiIconName;
  query: string;
}

const STARTERS: ReadonlyArray<StarterSpec> = [
  { key: "help", icon: "command", query: "/help" },
  { key: "setting", icon: "settings", query: "/setting" },
  { key: "note", icon: "note", query: "/note " },
  { key: "model", icon: "model", query: "/model" },
];

export function StarterActionsLine({ visible, onPickQuery }: Props): ReactElement | null {
  const p = useI18n().palette;
  if (!visible) return null;

  return (
    <div className="kn-panel-shell overflow-hidden rounded-t-none border-t-0 px-3 py-3">
      <div className="mb-2 px-1">
        <div className="text-[11px] font-semibold uppercase tracking-[0.12em] text-[color:var(--kn-text-faint)]">
          {p.starterTitle}
        </div>
        <div className="mt-0.5 truncate text-[11px] text-[color:var(--kn-text-muted)]">
          {p.starterSubtitle}
        </div>
      </div>

      <div className="grid grid-cols-2 gap-2 sm:grid-cols-4">
        {STARTERS.map(({ key, icon, query }) => {
          const action = p.starterActions[key];
          return (
            <button
              key={key}
              type="button"
              onClick={() => onPickQuery(query)}
              className="kn-quick-start w-full"
            >
              <span className="flex h-8 w-8 shrink-0 items-center justify-center rounded-[8px] border border-white/10 bg-white/[0.035] text-[color:var(--kn-accent)]">
                <UiIcon name={icon} className="h-4 w-4" />
              </span>
              <span className="min-w-0 text-left">
                <span className="block truncate text-xs font-semibold text-[color:var(--kn-text-soft)]">
                  {action.label}
                </span>
                <span className="block truncate text-[10px] text-[color:var(--kn-text-faint)]">
                  {action.description}
                </span>
              </span>
            </button>
          );
        })}
      </div>
    </div>
  );
}
