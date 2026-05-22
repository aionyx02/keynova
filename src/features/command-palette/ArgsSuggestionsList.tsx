// REF.2.P4 — Args-phase dropdown (rendered when an exact command match has
// returned suggestion strings and the user has typed past the trailing
// space). Mouse selection mirrors the keyboard path: clicking fills the
// query, hover updates the selected index for parity with Tab/Enter.

interface Props {
  cmdName: string;
  suggestions: string[];
  selectedIndex: number;
  onSelect: (arg: string, index: number) => void;
  onHover: (index: number) => void;
}

export function ArgsSuggestionsList({
  suggestions,
  selectedIndex,
  onSelect,
  onHover,
}: Props) {
  return (
    <div className="bg-gray-900/95 backdrop-blur-md rounded-b-xl shadow-2xl overflow-hidden">
      <ul className="max-h-[220px] overflow-y-auto py-1">
        {suggestions.map((arg, i) => (
          <li
            key={arg}
            onMouseDown={() => onSelect(arg, i)}
            onMouseEnter={() => onHover(i)}
            className={`flex items-center gap-2 px-4 py-2 cursor-pointer text-sm font-mono transition-colors ${
              i === selectedIndex ? "bg-blue-600/70 text-white" : "text-gray-300 hover:bg-white/8"
            }`}
          >
            {arg}
          </li>
        ))}
      </ul>
      <div className="border-t border-gray-700/50 px-4 py-1.5 text-[11px] text-gray-600 flex justify-between">
        <span>↑↓ 選擇</span>
        <span>Tab 填入</span>
        <span>Enter 執行</span>
      </div>
    </div>
  );
}
