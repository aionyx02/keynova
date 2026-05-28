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

const FEATURE_DESCRIPTIONS: Record<string, string> = {
  "features.ai": "需要 Ollama 或 API Key",
  "features.agent": "需要 Ollama 或支援工具呼叫的模型",
  "features.translation": "Google Translate 免費 API",
  "features.notes": "內建筆記與 LazyVim 整合",
  "features.history": "剪貼簿歷史記錄",
  "features.calculator": "即時運算機",
  "features.system": "系統資訊與控制",
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

function sectionDisplayLabel(section: string): string {
  const mapped = SECTION_LABELS[section];
  if (mapped) return mapped;
  if (!section) return section;
  return section.charAt(0).toUpperCase() + section.slice(1);
}

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
  const inputRefs = useRef<Array<HTMLElement | null>>([]);

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
    e: React.KeyboardEvent<HTMLElement>,
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
      const input = e.currentTarget as HTMLInputElement;
      if (isHotkey || (input.selectionStart === 0 && input.selectionEnd === 0)) {
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
        const input = e.currentTarget as HTMLInputElement;
        const len = input.value.length;
        if (input.selectionStart === len && input.selectionEnd === len) {
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

  function captureHotkey(e: React.KeyboardEvent<HTMLElement>, key: string) {
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
    <div className="kn-panel-shell overflow-hidden rounded-t-none border-t-0">
      <div className="relative border-b border-[color:var(--kn-border)] bg-white/[0.015]">
        <div className="pointer-events-none absolute inset-y-0 left-0 z-10 w-6 bg-gradient-to-r from-[color:var(--kn-panel-bg)] to-transparent" />
        <div className="pointer-events-none absolute inset-y-0 right-0 z-10 w-6 bg-gradient-to-l from-[color:var(--kn-panel-bg)] to-transparent" />
        <div className="setting-tabs-scroll overflow-x-auto overflow-y-hidden">
          <div className="flex min-w-max items-center gap-0.5 px-2">
            {sections.map((section) => (
              <button
                key={section}
                onClick={() => setActiveSection(section)}
                className={`shrink-0 border-b-2 px-3 py-2 text-[12px] font-semibold whitespace-nowrap transition-colors ${
                  activeSection === section
                    ? "border-[color:var(--kn-accent)] text-[color:var(--kn-text)]"
                    : "border-transparent text-[color:var(--kn-text-faint)] hover:text-[color:var(--kn-text-soft)]"
                }`}
              >
                {sectionDisplayLabel(section)}
              </button>
            ))}
          </div>
        </div>
      </div>

      <div className="px-4 pt-2 pb-0">
        <span className="text-[10px] text-[color:var(--kn-text-faint)]">
          {SECTION_EFFECT_HINT[activeSection] ?? "Applies after the next relevant action."}
        </span>
      </div>

      <div className="kn-scroll max-h-[260px] overflow-y-auto px-4 py-3 space-y-3">
        {visible.length === 0 && (
          <p className="py-4 text-center text-xs text-[color:var(--kn-text-faint)]">
            No settings in this section.
          </p>
        )}
        {visible.map(({ key, value, sensitive }, rowIdx) => {
          const fieldSchema = schema.find((item) => item.key === key);
          const label = fieldSchema?.label ?? key.split(".").slice(1).join(".");
          const displayValue = edits[key] ?? value;
          const isHotkey =
            fieldSchema?.value_type === "hotkey" || key.startsWith("hotkeys.");
          const isSecret = Boolean(sensitive || fieldSchema?.sensitive);
          const isBoolean = fieldSchema?.value_type === "boolean";
          const description = FEATURE_DESCRIPTIONS[key];

          if (isBoolean) {
            const isOn = displayValue === "true";
            return (
              <div key={key} className="flex items-center gap-3 py-0.5">
                <div className="flex-1 min-w-0">
                  <div className="truncate text-xs text-[color:var(--kn-text-soft)]">{label}</div>
                  {description && (
                    <div className="truncate text-[10px] text-[color:var(--kn-text-faint)]">{description}</div>
                  )}
                </div>
                <button
                  ref={(el) => { inputRefs.current[rowIdx] = el; }}
                  role="switch"
                  aria-checked={isOn}
                  onClick={() => void saveValue(key, isOn ? "false" : "true")}
                  onKeyDown={(e) => {
                    if (e.key === "ArrowDown") { e.preventDefault(); inputRefs.current[rowIdx + 1]?.focus(); }
                    if (e.key === "ArrowUp") { e.preventDefault(); inputRefs.current[rowIdx - 1]?.focus(); }
                    if (e.key === " " || e.key === "Enter") { e.preventDefault(); void saveValue(key, isOn ? "false" : "true"); }
                  }}
                  className={`relative inline-flex h-5 w-9 shrink-0 items-center rounded-full border transition-colors ${
                    isOn
                      ? "border-[color:rgba(138,168,255,0.24)] bg-[color:var(--kn-accent)]"
                      : "border-[color:var(--kn-border)] bg-white/[0.08]"
                  } ${saving === key ? "opacity-60" : ""}`}
                >
                  <span
                    className={`inline-block h-3.5 w-3.5 transform rounded-full bg-white shadow transition-transform ${
                      isOn ? "translate-x-[18px]" : "translate-x-[2px]"
                    }`}
                  />
                </button>
                {saving === key && (
                  <span className="shrink-0 text-[10px] text-[color:var(--kn-text-muted)]">Saving</span>
                )}
                {savedKey === key && saving !== key && (
                  <span className="shrink-0 text-[10px] text-[color:var(--kn-success)]">Saved</span>
                )}
              </div>
            );
          }

          return (
            <div key={key} className="flex items-center gap-3">
              <label className="w-44 shrink-0 truncate text-xs text-[color:var(--kn-text-muted)]">{label}</label>
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
                className={`kn-field flex-1 px-2 py-1 text-sm ${
                  isHotkey ? "cursor-pointer" : ""
                } ${saving === key ? "opacity-60" : ""}`}
                spellCheck={false}
              />
              {saving === key && (
                <span className="shrink-0 text-[10px] text-[color:var(--kn-text-muted)]">Saving</span>
              )}
              {savedKey === key && saving !== key && (
                <span className="shrink-0 text-[10px] text-[color:var(--kn-success)]">Saved</span>
              )}
            </div>
          );
        })}
      </div>

      <div className="kn-panel-footer">
        <span>%APPDATA%\Keynova\config.toml</span>
        {saveError ? (
          <span className="ml-2 truncate text-[color:var(--kn-danger)]">{saveError}</span>
        ) : (
          reloadNotice && <span className="ml-2 truncate text-[color:var(--kn-success)]">{reloadNotice}</span>
        )}
      </div>
    </div>
  );
}
