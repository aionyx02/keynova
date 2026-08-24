import React, { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { PanelProps } from "../../types/panel";
import type { SettingEntry, SettingSchema } from "./settingTypes";
import { SettingRow, type SettingControlKind } from "./SettingRow";
import { useI18n } from "../../i18n/useI18n";
import { fmt } from "../../i18n/format";

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
  "features",
  "ai",
  "agent",
  "translation",
  "notes",
  "history",
  "system",
  "performance",
];

function sectionDisplayLabel(section: string, labels: Record<string, string>): string {
  const mapped = labels[section];
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
  const s = useI18n().settings;
  const initialDraft = parseInitialArgs(initialArgs);
  const [entries, setEntries] = useState<SettingEntry[]>([]);
  const [activeSection, setActiveSection] = useState<Section>(
    initialDraft.key?.split(".")[0] ?? "hotkeys",
  );
  const [schema, setSchema] = useState<SettingSchema[]>([]);
  const [filter, setFilter] = useState("");
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
  const filterRef = useRef<HTMLInputElement>(null);

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
          ? fmt(s.reloadedFrom, { source: event.payload.source })
          : fmt(s.reloadedCount, { count, source: event.payload.source }),
      );
    });
    const unlistenFailed = listen<ConfigReloadFailedPayload>("config-reload-failed", (event) => {
      setReloadNotice(null);
      setSaveError(
        fmt(s.reloadFailed, { source: event.payload.source, error: event.payload.error }),
      );
    });
    return () => {
      unlistenReload.then((fn) => fn());
      unlistenFailed.then((fn) => fn());
    };
  }, [loadSettings, s]);

  const sections =
    schema.length > 0
      ? Array.from(new Set(schema.map((entry) => entry.section)))
      : DEFAULT_SECTIONS;

  const query = filter.trim().toLowerCase();
  const filtering = query.length > 0;
  const schemaFor = useCallback((key: string) => schema.find((item) => item.key === key), [schema]);
  const rows = filtering
    ? entries.filter((entry) => {
        const label = schemaFor(entry.key)?.label ?? "";
        return entry.key.toLowerCase().includes(query) || label.toLowerCase().includes(query);
      })
    : entries.filter((entry) => entry.key.startsWith(`${activeSection}.`));

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
    kind: SettingControlKind,
  ) {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      inputRefs.current[rowIdx + 1]?.focus();
      return;
    }
    if (e.key === "ArrowUp") {
      e.preventDefault();
      if (rowIdx === 0) filterRef.current?.focus();
      else inputRefs.current[rowIdx - 1]?.focus();
      return;
    }
    if (e.key === "ArrowLeft") {
      if (kind === "text") {
        const input = e.currentTarget as HTMLInputElement;
        if (input.selectionStart !== 0 || input.selectionEnd !== 0) return;
      }
      e.preventDefault();
      if (!filtering) switchSection(-1);
      return;
    }
    if (e.key === "ArrowRight") {
      if (kind === "text") {
        const input = e.currentTarget as HTMLInputElement;
        const len = input.value.length;
        if (input.selectionStart !== len || input.selectionEnd !== len) return;
      }
      e.preventDefault();
      if (!filtering) switchSection(1);
      return;
    }
    if (kind === "hotkey") {
      captureHotkey(e, key);
      return;
    }
    if (kind === "toggle") {
      if (e.key === " " || e.key === "Enter") {
        e.preventDefault();
        void saveValue(key, displayValue === "true" ? "false" : "true");
      }
      return;
    }
    if (kind === "text" && e.key === "Enter") {
      e.preventDefault();
      void saveValue(key, displayValue);
    }
  }

  function handleChange(key: string, value: string) {
    setEdits((prev) => ({ ...prev, [key]: value }));
  }

  async function saveValue(key: string, newValue: string) {
    const entry = entries.find((item) => item.key === key);
    const isSensitive = Boolean(entry?.sensitive || schemaFor(key)?.sensitive);
    if (isSensitive && !newValue.trim()) return;
    if (newValue === originalRef.current[key]) return;
    setSaving(key);
    setSaveError(null);
    try {
      await ipcDispatch("setting.set", { key, value: newValue });
      // Reflect "stored" for secrets with the same mask the backend returns, so
      // the row shows it as configured (the raw secret is never held in state).
      const storedValue = isSensitive ? "********" : newValue;
      originalRef.current[key] = storedValue;
      setEdits((prev) => {
        if (!(key in prev)) return prev;
        const next = { ...prev };
        delete next[key];
        return next;
      });
      setEntries((prev) =>
        prev.map((item) => (item.key === key ? { ...item, value: storedValue } : item)),
      );
      setReloadNotice(fmt(s.applied, { key }));
      setSavedKey(key);
      if (savedFlashRef.current) clearTimeout(savedFlashRef.current);
      savedFlashRef.current = setTimeout(() => setSavedKey(null), 1500);
    } catch (err) {
      setSaveError(String(err));
    } finally {
      setSaving(null);
    }
  }

  async function resetValue(key: string, defaultValue: string) {
    setEdits((prev) => {
      if (!(key in prev)) return prev;
      const next = { ...prev };
      delete next[key];
      return next;
    });
    await saveValue(key, defaultValue);
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
                onClick={() => {
                  setActiveSection(section);
                  setFilter("");
                }}
                className={`shrink-0 border-b-2 px-3 py-2 text-[12px] font-semibold whitespace-nowrap transition-colors ${
                  !filtering && activeSection === section
                    ? "border-[color:var(--kn-accent)] text-[color:var(--kn-text)]"
                    : "border-transparent text-[color:var(--kn-text-faint)] hover:text-[color:var(--kn-text-soft)]"
                }`}
              >
                {sectionDisplayLabel(section, s.sectionLabels)}
              </button>
            ))}
          </div>
        </div>
      </div>

      <div className="flex items-center gap-2 px-4 pt-2 pb-0">
        <input
          ref={filterRef}
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "ArrowDown") {
              e.preventDefault();
              inputRefs.current[0]?.focus();
            } else if (e.key === "Escape" && filter) {
              e.preventDefault();
              setFilter("");
            }
          }}
          placeholder={s.filterPlaceholder}
          className="kn-field flex-1 px-2 py-1 text-[11px]"
          spellCheck={false}
        />
        <span className="shrink-0 text-[10px] text-[color:var(--kn-text-faint)]">
          {filtering
            ? fmt(s.matchCount, { count: rows.length })
            : (s.sectionHints[activeSection] ?? s.defaultHint)}
        </span>
      </div>

      <div className="kn-scroll max-h-[260px] space-y-3 overflow-y-auto px-4 py-3">
        {rows.length === 0 && (
          <p className="py-4 text-center text-xs text-[color:var(--kn-text-faint)]">
            {filtering ? s.noMatch : s.noneInSection}
          </p>
        )}
        {rows.map((entry, rowIdx) => {
          const isSensitive = Boolean(entry.sensitive || schemaFor(entry.key)?.sensitive);
          const secretIsSet = isSensitive && entry.value.length > 0;
          // Keep the secret input empty so typing produces a clean key (never
          // appended onto the mask); the "Set" badge signals it's configured.
          const displayValue = isSensitive
            ? (edits[entry.key] ?? "")
            : (edits[entry.key] ?? entry.value);
          return (
            <SettingRow
              key={entry.key}
              entry={entry}
              fieldSchema={schemaFor(entry.key)}
              displayValue={displayValue}
              rowIdx={rowIdx}
              saving={saving === entry.key}
              saved={savedKey === entry.key && saving !== entry.key}
              secretIsSet={secretIsSet}
              showSection={filtering}
              registerRef={(el) => {
                inputRefs.current[rowIdx] = el;
              }}
              onChange={handleChange}
              onSave={(key, value) => void saveValue(key, value)}
              onReset={(key, defaultValue) => void resetValue(key, defaultValue)}
              onBlur={(key) => void handleBlur(key)}
              onKeyDown={handleInputKeyDown}
            />
          );
        })}
      </div>

      <div className="kn-panel-footer">
        <span>%APPDATA%\Keynova\config.toml</span>
        {saveError ? (
          <span className="ml-2 truncate text-[color:var(--kn-danger)]">{saveError}</span>
        ) : (
          reloadNotice && (
            <span className="ml-2 truncate text-[color:var(--kn-success)]">{reloadNotice}</span>
          )
        )}
      </div>
    </div>
  );
}
