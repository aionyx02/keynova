import { UiIcon } from "../../components/icons/UiIcon";
import { useI18n } from "../../i18n/useI18n";
import type { CommandMeta } from "../../hooks/useCommands";

interface Props {
  commands: CommandMeta[];
  selectedIndex: number;
  onSelect: (name: string) => void;
  onHover: (index: number) => void;
}

export function CommandSuggestions({ commands, selectedIndex, onSelect, onHover }: Props) {
  const p = useI18n().palette;
  if (commands.length === 0) return null;

  return (
    <div className="kn-panel-shell rounded-t-none border-t-0 overflow-hidden">
      <ul className="kn-scroll max-h-[360px] space-y-1 overflow-y-auto px-2 py-2">
        {commands.map((command, index) => {
          const isSelected = index === selectedIndex;

          return (
            <li
              key={command.name}
              ref={(element) => {
                if (isSelected && element) {
                  element.scrollIntoView({ block: "nearest" });
                }
              }}
              data-selected={isSelected ? "true" : "false"}
              onMouseDown={() => onSelect(command.name)}
              onMouseEnter={() => onHover(index)}
              className="kn-result-row flex cursor-pointer items-center gap-3 px-3 py-2.5"
            >
              <span className="flex h-10 w-10 shrink-0 items-center justify-center rounded-[8px] border border-[color:rgba(138,168,255,0.22)] bg-[color:var(--kn-accent-wash)] text-[color:var(--kn-accent)]">
                <UiIcon name="command" className="h-[18px] w-[18px]" />
              </span>
              <div className="min-w-0 flex-1">
                <div className="flex items-center gap-2">
                  <span className="truncate text-sm font-semibold text-[color:var(--kn-text)]">
                    /{command.name}
                  </span>
                  {command.args_hint && (
                    <span className="truncate font-mono text-[11px] text-[color:var(--kn-text-muted)]">
                      {command.args_hint}
                    </span>
                  )}
                </div>
                <div className="mt-1 truncate text-[11px] text-[color:var(--kn-text-muted)]">
                  {command.description}
                </div>
              </div>
            </li>
          );
        })}
      </ul>

      <div className="kn-panel-footer">
        <span className="flex items-center gap-1.5">
          <span className="kn-kbd">Up/Down</span>
          <span>{p.move}</span>
        </span>
        <span className="flex items-center gap-1.5">
          <span className="kn-kbd">Enter</span>
          <span>{p.run}</span>
        </span>
        <span className="flex items-center gap-1.5">
          <span className="kn-kbd">Esc</span>
          <span>{p.close}</span>
        </span>
      </div>
    </div>
  );
}
