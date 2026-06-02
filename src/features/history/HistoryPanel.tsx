import { useCallback, useEffect, useRef, useState } from "react";
import { useHistory } from "../../hooks/useHistory";
import { useI18n } from "../../i18n/useI18n";
import type { ClipboardEntry } from "../../hooks/useHistory";
import type { PanelProps } from "../../types/panel";

export function HistoryPanel({ onClose }: PanelProps) {
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
      search(value)
        .then(setFiltered)
        .catch(() => {});
    }, 200);
  }

  async function copyEntry(entry: ClipboardEntry) {
    await navigator.clipboard.writeText(entry.content);
    setCopiedId(entry.id);
    setTimeout(() => setCopiedId(null), 1200);
  }

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLInputElement>) => {
      if (e.key === "Escape") {
        e.preventDefault();
        onClose();
        return;
      }
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setSelected((i) => Math.min(i + 1, filtered.length - 1));
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        setSelected((i) => Math.max(i - 1, 0));
      } else if (e.key === "Enter") {
        const entry = filtered[selected];
        if (entry) void copyEntry(entry);
      }
    },
    [filtered, selected, onClose],
  );

  const displayList = query ? filtered : entries;

  return (
    <div
      className="kn-panel-shell flex flex-col rounded-t-none border-t-0"
      style={{ maxHeight: 400 }}
    >
      {/* Header */}
      <div className="kn-panel-header">
        <span className="kn-panel-title">{t.history.title}</span>
        <button onClick={() => void clearAll()} className="kn-button py-1 text-[10px]">
          {t.history.clearAll}
        </button>
      </div>

      {/* Search */}
      <div className="border-b border-[color:var(--kn-border)] px-4 py-2">
        <input
          ref={inputRef}
          value={query}
          onChange={(e) => handleSearch(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder={t.history.searchPlaceholder}
          className="kn-field w-full py-1.5 text-sm"
        />
      </div>

      {/* Entries */}
      <div className="kn-scroll flex-1 overflow-y-auto py-1">
        {displayList.length === 0 && (
          <p className="py-6 text-center text-xs text-[color:var(--kn-text-faint)]">
            {t.history.empty}
          </p>
        )}
        {displayList.map((entry, i) => (
          <div
            key={entry.id}
            onMouseEnter={() => setSelected(i)}
            className={`flex items-start gap-2 px-4 py-2 group transition-colors ${
              i === selected ? "bg-[color:var(--kn-accent-wash)]" : "hover:bg-white/[0.045]"
            }`}
          >
            {entry.pinned && <span className="shrink-0 text-[9px] text-amber-400 mt-0.5">📌</span>}
            <button
              onClick={() => void copyEntry(entry)}
              className="flex-1 truncate text-left font-mono text-xs leading-5 text-[color:var(--kn-text-soft)]"
              title={entry.content}
            >
              {entry.content.slice(0, 200)}
            </button>
            <div className="shrink-0 flex gap-1.5 opacity-0 group-hover:opacity-100 transition-opacity">
              <button
                onClick={() => void copyEntry(entry)}
                className="text-[9px] text-[color:var(--kn-text-faint)] hover:text-[color:var(--kn-accent)]"
              >
                {copiedId === entry.id ? "✓" : t.history.paste}
              </button>
              <button
                onClick={() => void pinEntry(entry.id, !entry.pinned)}
                className="text-[9px] text-[color:var(--kn-text-faint)] hover:text-amber-300"
              >
                {entry.pinned ? t.history.unpin : t.history.pin}
              </button>
              <button
                onClick={() => void deleteEntry(entry.id)}
                className="text-[9px] text-[color:var(--kn-text-faint)] hover:text-[color:var(--kn-danger)]"
              >
                {t.history.delete}
              </button>
            </div>
          </div>
        ))}
      </div>

      <div className="kn-panel-footer text-[10px]">
        <span>↑↓ 選擇</span>
        <span>Enter 複製</span>
        <span>Esc 關閉</span>
      </div>
    </div>
  );
}
