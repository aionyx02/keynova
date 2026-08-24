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
    <div className="kn-panel-shell overflow-hidden rounded-t-none border-t-0 px-2 py-2">
      <div className="mb-1.5 px-1.5 text-[11px] text-[color:var(--kn-text-faint)]">
        {p.starterTitle}
      </div>

      <div className="grid grid-cols-2 gap-1 sm:grid-cols-4">
        {STARTERS.map(({ key, icon, query }) => {
          const action = p.starterActions[key];
          return (
            <button
              key={key}
              type="button"
              onClick={() => onPickQuery(query)}
              className="kn-quick-start w-full"
            >
              <UiIcon
                name={icon}
                className="h-[17px] w-[17px] shrink-0 text-[color:var(--kn-accent)]"
              />
              <span className="min-w-0 text-left">
                <span className="block truncate text-[12.5px] text-[color:var(--kn-text-soft)]">
                  {action.label}
                </span>
                <span className="block truncate text-[10.5px] text-[color:var(--kn-text-faint)]">
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
