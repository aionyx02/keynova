import { useI18n } from "../../i18n/useI18n";

interface Props {
  cmdName: string;
  suggestions: string[];
  selectedIndex: number;
  onSelect: (arg: string, index: number) => void;
  onHover: (index: number) => void;
}

export function ArgsSuggestionsList({ suggestions, selectedIndex, onSelect, onHover }: Props) {
  const p = useI18n().palette;
  return (
    <div className="kn-panel-shell rounded-t-none border-t-0 overflow-hidden">
      <ul className="kn-scroll max-h-[220px] space-y-1 overflow-y-auto px-2 py-2">
        {suggestions.map((arg, index) => {
          const isSelected = index === selectedIndex;

          return (
            <li
              key={arg}
              data-selected={isSelected ? "true" : "false"}
              onMouseDown={() => onSelect(arg, index)}
              onMouseEnter={() => onHover(index)}
              className="kn-result-row cursor-pointer px-3 py-2.5 font-mono text-sm text-[color:var(--kn-text-soft)] transition-all duration-150"
            >
              {arg}
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
          <span className="kn-kbd">Tab</span>
          <span>{p.complete}</span>
        </span>
        <span className="flex items-center gap-1.5">
          <span className="kn-kbd">Enter</span>
          <span>{p.apply}</span>
        </span>
      </div>
    </div>
  );
}
