import { useCallback, useEffect, useRef, useState } from "react";
import { useHistory } from "../hooks/useHistory";
import { useI18n } from "../i18n/useI18n";
import type { ClipboardEntry } from "../hooks/useHistory";

export function HistoryPanel() {
  const t = useI18n();
  const { entries, search, deleteEntry, pinEntry, clearAll } = useHistory();
  const [query, setQuery] = useState("");
  const [filtered, setFiltered] = useState<ClipboardEntry[]>([]);
  const [selected, setSelected] = useState(0);
  const [copiedId, setCopiedId] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  function handleSearch(value: string) {
    setQuery(value);
    setSelected(0);
    if (debounceRef.current) clearTimeout(debounceRef.current);
    if (!value.trim()) {
      setFiltered(entries);
      return;
    }
    debounceRef.current = setTimeout(() => {
      search(value).then(setFiltered).catch(() => {});
    }, 200);
  }

  async function copyEntry(entry: ClipboardEntry) {
    await navigator.clipboard.writeText(entry.content);
    setCopiedId(entry.id);
    setTimeout(() => setCopiedId(null), 1200);
  }

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLInputElement>) => {
      if (e.key === "ArrowDown") { e.preventDefault(); setSelected((i) => Math.min(i + 1, filtered.length - 1)); }
      else if (e.key === "ArrowUp") { e.preventDefault(); setSelected((i) => Math.max(i - 1, 0)); }
      else if (e.key === "Enter") {
        const entry = filtered[selected];
        if (entry) void copyEntry(entry);
      }
    },
    [filtered, selected],
  );

  const displayList = query ? filtered : entries;

  return (
    <div className="bg-gray-900/95 backdrop-blur-md rounded-b-xl shadow-2xl flex flex-col" style={{ maxHeight: 400 }}>
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-2 border-b border-gray-700/50">
        <span className="text-xs font-semibold text-blue-400 uppercase tracking-wide">{t.history.title}</span>
        <button
          onClick={() => void clearAll()}
          className="text-[10px] text-gray-600 hover:text-gray-400 transition-colors"
        >
          {t.history.clearAll}
        </button>
      </div>

      {/* Search */}
      <div className="px-4 py-2 border-b border-gray-700/30">
        <input
          ref={inputRef}
          value={query}
          onChange={(e) => handleSearch(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder={t.history.searchPlaceholder}
          className="w-full bg-gray-800/60 text-gray-200 text-sm rounded px-3 py-1.5 outline-none placeholder-gray-600"
        />
      </div>

      {/* Entries */}
      <div className="flex-1 overflow-y-auto py-1">
        {displayList.length === 0 && (
          <p className="text-center text-gray-700 text-xs py-6">{t.history.empty}</p>
        )}
        {displayList.map((entry, i) => (
          <div
            key={entry.id}
            onMouseEnter={() => setSelected(i)}
            className={`flex items-start gap-2 px-4 py-2 group transition-colors ${
              i === selected ? "bg-blue-600/20" : "hover:bg-white/5"
            }`}
          >
            {entry.pinned && (
              <span className="shrink-0 text-[9px] text-amber-400 mt-0.5">📌</span>
            )}
            <button
              onClick={() => void copyEntry(entry)}
              className="flex-1 text-left text-xs text-gray-300 truncate font-mono leading-5"
              title={entry.content}
            >
              {entry.content.slice(0, 200)}
            </button>
            <div className="shrink-0 flex gap-1.5 opacity-0 group-hover:opacity-100 transition-opacity">
              <button
                onClick={() => void copyEntry(entry)}
                className="text-[9px] text-gray-600 hover:text-blue-400"
              >
                {copiedId === entry.id ? "✓" : t.history.paste}
              </button>
              <button
                onClick={() => void pinEntry(entry.id, !entry.pinned)}
                className="text-[9px] text-gray-600 hover:text-amber-400"
              >
                {entry.pinned ? t.history.unpin : t.history.pin}
              </button>
              <button
                onClick={() => void deleteEntry(entry.id)}
                className="text-[9px] text-gray-600 hover:text-red-400"
              >
                {t.history.delete}
              </button>
            </div>
          </div>
        ))}
      </div>

      <div className="px-4 py-1.5 border-t border-gray-700/30 text-[10px] text-gray-700 flex justify-between">
        <span>↑↓ 選擇</span>
        <span>Enter 複製</span>
        <span>Esc 關閉</span>
      </div>
    </div>
  );
}