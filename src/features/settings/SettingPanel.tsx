// The settings form.
//
// It is the whole content of the settings window (`windows/SettingsWindow`) and
// is no longer reachable as a palette panel: settings is the one surface people
// read and scroll rather than type through, which a 700x50 hide-on-blur strip
// cannot host.
//
// Two things follow from being a window rather than a panel:
//
//   - No `initialArgs`. `/setting <key>` and `/setting <key> <value>` are
//     answered inline by the backend and never open anything, so bare
//     `/setting` — which carries nothing — is the only way in.
//   - No `config-reloaded` listener. Tauri's ACL gates `plugin:core:event|*`
//     per window and `capabilities/default.json` covers only `main`, so this
//     webview cannot subscribe to events. It is not load-bearing: every write
//     from this form updates local state directly, and the launcher (which does
//     hold the capability) still hears the broadcast and re-applies the theme.
//     An edit made to `config.toml` in an external editor while this window is
//     open is the case that no longer refreshes on its own.

import React, { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { SettingEntry, SettingSchema } from "./settingTypes";
import { SettingRow, type SettingControlKind } from "./SettingRow";
import { SettingSidebar } from "./SettingSidebar";
import { useI18n } from "../../i18n/useI18n";
import { fmt } from "../../i18n/format";

export interface SettingPanelProps {
  /**
   * Reports `launcher.theme` on load and after every save. The theme is written
   * onto <html>, which belongs to the window shell rather than to this form.
   */
  onThemeChange?: (value: string | undefined) => void;
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

type Section = string;

export function SettingPanel({ onThemeChange }: SettingPanelProps) {
  const s = useI18n().settings;
  const [entries, setEntries] = useState<SettingEntry[]>([]);
  const [activeSection, setActiveSection] = useState<Section>("hotkeys");
  const [schema, setSchema] = useState<SettingSchema[]>([]);
  const [filter, setFilter] = useState("");
  const [edits, setEdits] = useState<Record<string, string>>({});
  const [saving, setSaving] = useState<string | null>(null);
  const [savedKey, setSavedKey] = useState<string | null>(null);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [reloadNotice, setReloadNotice] = useState<string | null>(null);
  const originalRef = useRef<Record<string, string>>({});
  const savedFlashRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const inputRefs = useRef<Array<HTMLElement | null>>([]);
  const sectionRefs = useRef<Array<HTMLButtonElement | null>>([]);
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

  // The theme is the one setting that does not render as a row: it is written
  // onto <html>. Deriving it from `entries` means load and save feed the shell
  // through one path, and the effect only fires when the value actually moves.
  const themeValue = entries.find((entry) => entry.key === "launcher.theme")?.value;
  useEffect(() => {
    onThemeChange?.(themeValue);
  }, [themeValue, onThemeChange]);

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
      // Never yank focus out of the rail. Arrowing through sections switches
      // the content as it goes, and this effect fires on every one of those
      // switches — without the guard it would throw the caret into the rows
      // 50 ms into the first keypress and make the rail unusable.
      if (sectionRefs.current.some((el) => el === document.activeElement)) return;
      inputRefs.current[0]?.focus();
    }, 50);
    return () => window.clearTimeout(timer);
  }, [entries.length, activeSection]);

  function selectSection(section: string) {
    setActiveSection(section);
    setFilter("");
  }

  /** Moves the rail selection and keeps focus on it, so ArrowUp/ArrowDown read
   *  as one gesture rather than a move followed by a jump. */
  function moveSection(dir: 1 | -1) {
    const idx = sections.indexOf(activeSection);
    const nextIdx = Math.max(0, Math.min(sections.length - 1, idx + dir));
    const next = sections[nextIdx];
    if (!next) return;
    selectSection(next);
    sectionRefs.current[nextIdx]?.focus();
  }

  function focusActiveSection() {
    const idx = sections.indexOf(activeSection);
    sectionRefs.current[Math.max(0, idx)]?.focus();
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
    // ArrowLeft crosses into the rail, which is literally to the left. A text
    // field only gives the key up once the caret has nowhere further to go.
    // There is deliberately no ArrowRight counterpart: nothing sits to the
    // right of the rows, so the key stays with the caret.
    if (e.key === "ArrowLeft") {
      if (kind === "text") {
        const input = e.currentTarget as HTMLInputElement;
        if (input.selectionStart !== 0 || input.selectionEnd !== 0) return;
      }
      e.preventDefault();
      focusActiveSection();
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
    // Fills the window rather than sitting in one: no shell border, no radius,
    // and the row list — not a fixed 260 px — takes whatever height is left.
    <div className="flex h-full min-h-0 flex-col overflow-hidden">
      {/* The filter spans both panes because it searches every section, not
          the one the rail has selected. */}
      <div className="flex shrink-0 items-center gap-2 border-b border-[color:var(--kn-border)] px-4 py-2">
        <input
          ref={filterRef}
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "ArrowDown") {
              e.preventDefault();
              inputRefs.current[0]?.focus();
            } else if (e.key === "Escape" && filter) {
              // First Escape clears the filter, second closes the window.
              // `stopPropagation` is what keeps the window-level handler in
              // `SettingsWindow` from doing both at once.
              e.preventDefault();
              e.stopPropagation();
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

      <div className="flex min-h-0 flex-1">
        <SettingSidebar
          sections={sections}
          activeSection={activeSection}
          labels={s.sectionLabels}
          filtering={filtering}
          onSelect={selectSection}
          onMove={moveSection}
          onEnterRows={() => inputRefs.current[0]?.focus()}
          registerRef={(el, index) => {
            sectionRefs.current[index] = el;
          }}
        />

        <div className="kn-scroll min-h-0 flex-1 space-y-3 overflow-y-auto px-4 py-3">
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
      </div>

      <div className="kn-panel-footer shrink-0">
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
