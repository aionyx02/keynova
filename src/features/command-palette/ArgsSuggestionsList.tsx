interface Props {
  cmdName: string;
  suggestions: string[];
  selectedIndex: number;
  onSelect: (arg: string, index: number) => void;
  onHover: (index: number) => void;
}

export function ArgsSuggestionsList({ suggestions, selectedIndex, onSelect, onHover }: Props) {
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

      <div className="flex items-center justify-between border-t border-[color:var(--kn-border)] bg-[rgba(7,11,17,0.48)] px-4 py-2 text-[11px] text-[color:var(--kn-text-muted)]">
        <span className="flex items-center gap-1.5">
          <span className="kn-kbd">Up/Down</span>
          <span>move</span>
        </span>
        <span className="flex items-center gap-1.5">
          <span className="kn-kbd">Tab</span>
          <span>complete</span>
        </span>
        <span className="flex items-center gap-1.5">
          <span className="kn-kbd">Enter</span>
          <span>apply</span>
        </span>
      </div>
    </div>
  );
}
