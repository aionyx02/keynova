import React, { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { PanelProps } from "../types/panel";

interface SettingEntry {
  key: string;
  value: string;
  sensitive?: boolean;
}

interface SettingSchema {
  key: string;
  section: string;
  label: string;
  value_type: "string" | "integer" | "boolean" | "hotkey" | "secret";
  sensitive: boolean;
  options: string[];
}

interface ConfigReloadedPayload {
  source: string;
  changed_keys: string[];
}

interface ConfigReloadFailedPayload {
  source: string;
  error: string;
}

interface SettingDraftPayload {
  key?: string;
  value?: string;
}

async function ipcDispatch<T>(route: string, payload?: Record<string, unknown>): Promise<T> {
  return invoke<T>("cmd_dispatch", { route, payload: payload ?? null });
}

const DEFAULT_SECTIONS = [
  "hotkeys",
  "launcher",
  "search",
  "terminal",
  "mouse_control",
  "features",
  "ai",
  "agent",
  "translation",
  "notes",
  "history",
  "system",
];

const SECTION_LABELS: Record<string, string> = {
  hotkeys: "Hotkeys",
  launcher: "Launcher",
  search: "Search",
  terminal: "Terminal",
  mouse_control: "Mouse",
  features: "Features",
  ai: "AI",
  agent: "Agent",
  translation: "Translation",
  notes: "Notes",
  history: "History",
  system: "System",
};

const SECTION_EFFECT_HINT: Record<string, string> = {
  hotkeys: "Applies after shortcut reload.",
  launcher: "Used by the command palette immediately.",
  search: "Changes search behavior and index selection.",
  terminal: "Takes effect the next time a terminal opens.",
  mouse_control: "Used by mouse control mode.",
  features: "Toggles feature visibility.",
  ai: "Affects provider and model configuration.",
  agent: "Controls agent web search and memory behavior.",
  translation: "Used by translation requests.",
  notes: "Affects note storage and editor integration.",
  history: "Controls clipboard history retention.",
  system: "Controls system panel capabilities.",
};

function parseInitialArgs(initialArgs?: string): SettingDraftPayload {
  const value = initialArgs?.trim();
  if (!value) return {};
  try {
    const parsed = JSON.parse(value) as SettingDraftPayload;
    if (parsed && parsed.key) return parsed;
  } catch {
    return { key: value };
  }
  return {};
}

type Section = string;

export function SettingPanel({ initialArgs }: PanelProps) {
  const initialDraft = parseInitialArgs(initialArgs);
  const [entries, setEntries] = useState<SettingEntry[]>([]);
  const [activeSection, setActiveSection] = useState<Section>(
    initialDraft.key?.split(".")[0] ?? "hotkeys",
  );
  const [schema, setSchema] = useState<SettingSchema[]>([]);
  const [edits, setEdits] = useState<Record<string, string>>(() =>
    initialDraft.key && initialDraft.value !== undefined
      ? { [initialDraft.key]: initialDraft.value }
      : {},
  );
  const [saving, setSaving] = useState<string | null>(null);
  const [savedKey, setSavedKey] = useState<string | null>(null);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [reloadNotice, setReloadNotice] = useState<string | null>(null);
  const originalRef = useRef<Record<string, string>>({});
  const savedFlashRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const inputRefs = useRef<Array<HTMLInputElement | null>>([]);

  const loadSettings = useCallback(async () => {
    if (!window.__TAURI_INTERNALS__) return;
    const [data, schemaData] = await Promise.all([
      ipcDispatch<SettingEntry[]>("setting.list_all"),
      ipcDispatch<SettingSchema[]>("setting.schema"),
    ]);
    setSchema(schemaData);
    setEntries(data);
    const orig: Record<string, string> = {};
    data.forEach(({ key, value }) => {
      orig[key] = value;
    });
    originalRef.current = orig;
  }, [setEntries, setSchema]);

  useEffect(() => {
    const timer = window.setTimeout(() => {
      void loadSettings().catch(() => {});
    }, 0);
    return () => window.clearTimeout(timer);
  }, [loadSettings]);

  useEffect(() => {
    if (!window.__TAURI_INTERNALS__) return;
    const unlistenReload = listen<ConfigReloadedPayload>("config-reloaded", (event) => {
      void loadSettings().catch(() => {});
      const count = event.payload.changed_keys.length;
      setSaveError(null);
      setReloadNotice(
        count === 0
          ? `Reloaded from ${event.payload.source}`
          : `Reloaded ${count} setting(s) from ${event.payload.source}`,
      );
    });
    const unlistenFailed = listen<ConfigReloadFailedPayload>(
      "config-reload-failed",
      (event) => {
        setReloadNotice(null);
        setSaveError(`Reload failed (${event.payload.source}): ${event.payload.error}`);
      },
    );
    return () => {
      unlistenReload.then((fn) => fn());
      unlistenFailed.then((fn) => fn());
    };
  }, [loadSettings]);

  const sections =
    schema.length > 0 ? Array.from(new Set(schema.map((entry) => entry.section))) : DEFAULT_SECTIONS;
  const visible = entries.filter((entry) => entry.key.startsWith(`${activeSection}.`));

  useEffect(() => {
    if (entries.length === 0) return;
    const timer = window.setTimeout(() => {
      inputRefs.current[0]?.focus();
    }, 50);
    return () => window.clearTimeout(timer);
  }, [entries.length, activeSection]);

  function switchSection(dir: 1 | -1) {
    const idx = sections.indexOf(activeSection);
    const next = sections[Math.max(0, Math.min(sections.length - 1, idx + dir))];
    if (next && next !== activeSection) setActiveSection(next);
  }

  function handleInputKeyDown(
    e: React.KeyboardEvent<HTMLInputElement>,
    key: string,
    rowIdx: number,
    displayValue: string,
    isHotkey: boolean,
  ) {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      inputRefs.current[rowIdx + 1]?.focus();
      return;
    }
    if (e.key === "ArrowUp") {
      e.preventDefault();
      inputRefs.current[rowIdx - 1]?.focus();
      return;
    }
    if (e.key === "ArrowLeft") {
      if (isHotkey || (e.currentTarget.selectionStart === 0 && e.currentTarget.selectionEnd === 0)) {
        e.preventDefault();
        switchSection(-1);
      }
      return;
    }
    if (e.key === "ArrowRight") {
      if (isHotkey) {
        e.preventDefault();
        switchSection(1);
      } else {
        const len = e.currentTarget.value.length;
        if (e.currentTarget.selectionStart === len && e.currentTarget.selectionEnd === len) {
          e.preventDefault();
          switchSection(1);
        }
      }
      return;
    }
    if (isHotkey) {
      captureHotkey(e, key);
      return;
    }
    if (e.key === "Enter") {
      e.preventDefault();
      void saveValue(key, displayValue);
    }
  }

  function handleChange(key: string, value: string) {
    setEdits((prev) => ({ ...prev, [key]: value }));
  }

  async function saveValue(key: string, newValue: string) {
    const entry = entries.find((item) => item.key === key);
    if (entry?.sensitive && !newValue.trim()) return;
    if (newValue === originalRef.current[key]) return;
    setSaving(key);
    setSaveError(null);
    try {
      await ipcDispatch("setting.set", { key, value: newValue });
      originalRef.current[key] = newValue;
      setEntries((prev) =>
        prev.map((item) => (item.key === key ? { ...item, value: newValue } : item)),
      );
      setReloadNotice(`Applied ${key}`);
      setSavedKey(key);
      if (savedFlashRef.current) clearTimeout(savedFlashRef.current);
      savedFlashRef.current = setTimeout(() => setSavedKey(null), 1500);
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
      <div className="flex border-b border-gray-700/50">
        {sections.map((section) => (
          <button
            key={section}
            onClick={() => setActiveSection(section)}
            className={`flex-1 py-2 text-xs font-medium transition-colors ${
              activeSection === section
                ? "text-blue-400 border-b-2 border-blue-400"
                : "text-gray-500 hover:text-gray-300"
            }`}
          >
            {SECTION_LABELS[section] ?? section}
          </button>
        ))}
      </div>

      <div className="px-4 pt-2 pb-0">
        <span className="text-[10px] text-gray-600">
          {SECTION_EFFECT_HINT[activeSection] ?? "Applies after the next relevant action."}
        </span>
      </div>

      <div className="max-h-[260px] overflow-y-auto px-4 py-3 space-y-3">
        {visible.length === 0 && (
          <p className="text-xs text-gray-600 text-center py-4">No settings in this section.</p>
        )}
        {visible.map(({ key, value, sensitive }, rowIdx) => {
          const fieldSchema = schema.find((item) => item.key === key);
          const label = fieldSchema?.label ?? key.split(".").slice(1).join(".");
          const displayValue = edits[key] ?? value;
          const isHotkey =
            fieldSchema?.value_type === "hotkey" || key.startsWith("hotkeys.");
          const isSecret = Boolean(sensitive || fieldSchema?.sensitive);
          return (
            <div key={key} className="flex items-center gap-3">
              <label className="w-44 shrink-0 text-xs text-gray-400 truncate">{label}</label>
              <input
                ref={(el) => {
                  inputRefs.current[rowIdx] = el;
                }}
                value={displayValue}
                readOnly={isHotkey}
                type={isSecret ? "password" : "text"}
                onChange={isHotkey ? undefined : (e) => handleChange(key, e.target.value)}
                onKeyDown={(e) => handleInputKeyDown(e, key, rowIdx, displayValue, isHotkey)}
                onBlur={isHotkey ? undefined : () => void handleBlur(key)}
                placeholder={
                  isHotkey
                    ? "Press the shortcut"
                    : isSecret
                      ? "Enter a new secret value"
                      : undefined
                }
                className={`flex-1 rounded bg-gray-800 px-2 py-1 text-sm text-gray-100 outline-none focus:ring-1 ${
                  isHotkey ? "focus:ring-violet-500 cursor-pointer" : "focus:ring-blue-500"
                } ${saving === key ? "opacity-60" : ""}`}
                spellCheck={false}
              />
              {saving === key && (
                <span className="text-[10px] text-gray-500 shrink-0">Saving</span>
              )}
              {savedKey === key && saving !== key && (
                <span className="text-[10px] text-emerald-400 shrink-0">Saved</span>
              )}
            </div>
          );
        })}
      </div>

      <div className="border-t border-gray-700/50 px-4 py-1.5 text-[11px] text-gray-600 flex justify-between">
        <span>%APPDATA%\Keynova\config.toml</span>
        {saveError ? (
          <span className="text-red-400 truncate ml-2">{saveError}</span>
        ) : (
          reloadNotice && <span className="text-emerald-400 truncate ml-2">{reloadNotice}</span>
        )}
      </div>
    </div>
  );
}
