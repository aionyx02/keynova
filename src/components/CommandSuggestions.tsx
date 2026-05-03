import type { CommandMeta } from "../hooks/useCommands";

interface Props {
  commands: CommandMeta[];
  selectedIndex: number;
  onSelect: (name: string) => void;
  onHover: (index: number) => void;
}

export function CommandSuggestions({ commands, selectedIndex, onSelect, onHover }: Props) {
  if (commands.length === 0) return null;

  return (
    <div className="bg-gray-900/95 backdrop-blur-md rounded-b-xl shadow-2xl overflow-hidden">
      <ul className="max-h-[352px] overflow-y-auto py-1">
        {commands.map((cmd, i) => (
          <li
            key={cmd.name}
            onMouseDown={() => onSelect(cmd.name)}
            onMouseEnter={() => onHover(i)}
            className={`flex items-center gap-3 px-4 py-2.5 cursor-pointer text-sm transition-colors ${
              i === selectedIndex
                ? "bg-blue-600/70 text-white"
                : "text-gray-300 hover:bg-white/8"
            }`}
          >
            <span className="shrink-0 rounded px-1.5 py-0.5 text-[10px] font-semibold uppercase tracking-wide bg-blue-500/30 text-blue-300">
              CMD
            </span>
            <span className="font-medium">/{cmd.name}</span>
            <span className={`ml-auto text-xs truncate ${i === selectedIndex ? "text-blue-200" : "text-gray-500"}`}>
              {cmd.description}
            </span>
          </li>
        ))}
      </ul>
      <div className="border-t border-gray-700/50 px-4 py-1.5 text-[11px] text-gray-600 flex justify-between">
        <span>↑↓ 選擇</span>
        <span>Enter 執行</span>
        <span>Esc 關閉</span>
      </div>
    </div>
  );
}