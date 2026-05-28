import { UiIcon } from "../../components/icons/UiIcon";
import type { CommandMeta } from "../../hooks/useCommands";

interface Props {
  commands: CommandMeta[];
  selectedIndex: number;
  onSelect: (name: string) => void;
  onHover: (index: number) => void;
}

export function CommandSuggestions({ commands, selectedIndex, onSelect, onHover }: Props) {
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
              className="kn-result-row flex cursor-pointer items-center gap-3 px-3 py-3 transition-all duration-150"
            >
              <span className="flex h-10 w-10 shrink-0 items-center justify-center rounded-[12px] border border-cyan-400/20 bg-cyan-400/10 text-cyan-200">
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

      <div className="flex items-center justify-between border-t border-[color:var(--kn-border)] bg-[rgba(7,11,17,0.48)] px-4 py-2 text-[11px] text-[color:var(--kn-text-muted)]">
        <span className="flex items-center gap-1.5">
          <span className="kn-kbd">Up/Down</span>
          <span>move</span>
        </span>
        <span className="flex items-center gap-1.5">
          <span className="kn-kbd">Enter</span>
          <span>run</span>
        </span>
        <span className="flex items-center gap-1.5">
          <span className="kn-kbd">Esc</span>
          <span>close</span>
        </span>
      </div>
    </div>
  );
}
