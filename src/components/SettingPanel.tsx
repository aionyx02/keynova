import React, { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface SettingEntry {
  key: string;
  value: string;
}

async function ipcDispatch<T>(route: string, payload?: Record<string, unknown>): Promise<T> {
  return invoke<T>("cmd_dispatch", { route, payload: payload ?? null });
}

const SECTIONS = ["hotkeys", "terminal", "launcher", "mouse_control"] as const;
type Section = (typeof SECTIONS)[number];

const SECTION_LABELS: Record<Section, string> = {
  hotkeys: "快捷鍵",
  terminal: "終端機",
  launcher: "啟動器",
  mouse_control: "滑鼠控制",
};

export function SettingPanel() {
  const [entries, setEntries] = useState<SettingEntry[]>([]);
  const [activeSection, setActiveSection] = useState<Section>("hotkeys");
  // Local edits: key → edited value (not yet saved)
  const [edits, setEdits] = useState<Record<string, string>>({});
  const [saving, setSaving] = useState<string | null>(null);
  const [saveError, setSaveError] = useState<string | null>(null);
  // Track original values to detect actual changes on blur
  const originalRef = useRef<Record<string, string>>({});

  useEffect(() => {
    if (!window.__TAURI_INTERNALS__) return;
    ipcDispatch<SettingEntry[]>("setting.list_all")
      .then((data) => {
        setEntries(data);
        const orig: Record<string, string> = {};
        data.forEach(({ key, value }) => { orig[key] = value; });
        originalRef.current = orig;
      })
      .catch(() => {});
  }, []);

  const visible = entries.filter((e) => e.key.startsWith(`${activeSection}.`));

  function handleChange(key: string, value: string) {
    setEdits((prev) => ({ ...prev, [key]: value }));
  }

  async function saveValue(key: string, newValue: string) {
    if (newValue === originalRef.current[key]) return;
    setSaving(key);
    setSaveError(null);
    try {
      await ipcDispatch("setting.set", { key, value: newValue });
      originalRef.current[key] = newValue;
      setEntries((prev) => prev.map((e) => (e.key === key ? { ...e, value: newValue } : e)));
    } catch (err) {
      setSaveError(String(err));
    } finally {
      setSaving(null);
    }
  }

  async function handleBlur(key: string) {
    const edited = edits[key];
    if (edited === undefined) return;
    await saveValue(key, edited);
  }

  function captureHotkey(e: React.KeyboardEvent<HTMLInputElement>, key: string) {
    e.preventDefault();
    if (["Control", "Alt", "Shift", "Meta"].includes(e.key)) return;
    const parts: string[] = [];
    if (e.ctrlKey) parts.push("Ctrl");
    if (e.altKey) parts.push("Alt");
    if (e.shiftKey) parts.push("Shift");
    if (e.metaKey) parts.push("Super");
    parts.push(e.key.length === 1 ? e.key.toUpperCase() : e.key);
    const combo = parts.join("+");
    handleChange(key, combo);
    void saveValue(key, combo);
  }

  return (
    <div className="bg-gray-900/95 backdrop-blur-md rounded-b-xl shadow-2xl overflow-hidden">
      {/* Section tabs */}
      <div className="flex border-b border-gray-700/50">
        {SECTIONS.map((sec) => (
          <button
            key={sec}
            onClick={() => setActiveSection(sec)}
            className={`flex-1 py-2 text-xs font-medium transition-colors ${
              activeSection === sec
                ? "text-blue-400 border-b-2 border-blue-400"
                : "text-gray-500 hover:text-gray-300"
            }`}
          >
            {SECTION_LABELS[sec]}
          </button>
        ))}
      </div>

      {/* Settings fields */}
      <div className="max-h-[280px] overflow-y-auto px-4 py-3 space-y-3">
        {visible.length === 0 && (
          <p className="text-xs text-gray-600 text-center py-4">無設定項目</p>
        )}
        {visible.map(({ key, value }) => {
          const label = key.split(".").slice(1).join(".");
          const displayValue = edits[key] ?? value;
          const isHotkey = key.startsWith("hotkeys.");
          return (
            <div key={key} className="flex items-center gap-3">
              <label className="w-40 shrink-0 text-xs text-gray-400 truncate">{label}</label>
              <input
                value={displayValue}
                readOnly={isHotkey}
                onChange={isHotkey ? () => {} : (e) => handleChange(key, e.target.value)}
                onKeyDown={isHotkey ? (e) => captureHotkey(e, key) : undefined}
                onBlur={isHotkey ? undefined : () => void handleBlur(key)}
                placeholder={isHotkey ? "點選後按組合鍵…" : undefined}
                className={`flex-1 rounded bg-gray-800 px-2 py-1 text-sm text-gray-100 outline-none focus:ring-1 ${
                  isHotkey ? "focus:ring-violet-500 cursor-pointer" : "focus:ring-blue-500"
                } ${saving === key ? "opacity-60" : ""}`}
                spellCheck={false}
              />
              {saving === key && (
                <span className="text-[10px] text-gray-500 shrink-0">儲存中…</span>
              )}
            </div>
          );
        })}
      </div>

      <div className="border-t border-gray-700/50 px-4 py-1.5 text-[11px] text-gray-600 flex justify-between">
        <span>失去焦點時自動儲存至 %APPDATA%\Keynova\config.toml</span>
        {saveError && <span className="text-red-400 truncate ml-2">{saveError}</span>}
      </div>
    </div>
  );
}