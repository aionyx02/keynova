import React from "react";
import type { SettingEntry, SettingSchema } from "./settingTypes";
import { useI18n } from "../../i18n/useI18n";

export type SettingControlKind = "text" | "hotkey" | "toggle" | "select";

interface SettingRowProps {
  entry: SettingEntry;
  fieldSchema?: SettingSchema;
  displayValue: string;
  rowIdx: number;
  saving: boolean;
  saved: boolean;
  /** True when this is a sensitive key that already has a stored value. */
  secretIsSet?: boolean;
  showSection: boolean;
  registerRef: (el: HTMLElement | null) => void;
  onChange: (key: string, value: string) => void;
  onSave: (key: string, value: string) => void;
  onReset: (key: string, defaultValue: string) => void;
  onBlur: (key: string) => void;
  onKeyDown: (
    e: React.KeyboardEvent<HTMLElement>,
    key: string,
    rowIdx: number,
    displayValue: string,
    kind: SettingControlKind,
  ) => void;
}

function StatusBadge({ saving, saved }: { saving: boolean; saved: boolean }) {
  // role=status + aria-live so the save outcome is announced; sighted users see
  // the same badge, AT users hear "Saving"/"Saved" without it.
  const s = useI18n().settings;
  const text = saving ? s.saving : saved ? s.savedFlash : "";
  const tone = saving ? "var(--kn-text-muted)" : "var(--kn-success)";
  return (
    <span
      role="status"
      aria-live="polite"
      className="shrink-0 text-[10px]"
      style={{ color: tone }}
    >
      {text}
    </span>
  );
}

export function SettingRow({
  entry,
  fieldSchema,
  displayValue,
  rowIdx,
  saving,
  saved,
  secretIsSet = false,
  showSection,
  registerRef,
  onChange,
  onSave,
  onReset,
  onBlur,
  onKeyDown,
}: SettingRowProps) {
  const s = useI18n().settings;
  const { key, sensitive } = entry;
  const label = fieldSchema?.label ?? key.split(".").slice(1).join(".");
  const isHotkey = fieldSchema?.value_type === "hotkey" || key.startsWith("hotkeys.");
  const isSecret = Boolean(sensitive || fieldSchema?.sensitive);
  const isBoolean = fieldSchema?.value_type === "boolean";
  const options = fieldSchema?.options ?? [];
  const hasOptions = !isBoolean && options.length > 0;
  const defaultValue = fieldSchema?.default_value;
  const isModified = defaultValue !== undefined && !isSecret && displayValue !== defaultValue;
  const section = key.split(".")[0];
  const description = s.featureDescriptions[key];

  const sectionTag = showSection ? (
    <span className="mr-1.5 rounded bg-white/[0.06] px-1 py-0.5 text-[9px] uppercase tracking-wide text-[color:var(--kn-text-faint)]">
      {section}
    </span>
  ) : null;

  const modifiedDot = isModified ? (
    <span
      className="ml-1 inline-block h-1.5 w-1.5 shrink-0 rounded-full bg-[color:var(--kn-accent)]"
      title={`Modified · default: ${defaultValue || "(empty)"}`}
    />
  ) : null;

  const resetButton =
    isModified && defaultValue !== undefined ? (
      <button
        onClick={() => onReset(key, defaultValue)}
        title={`Reset to default (${defaultValue || "empty"})`}
        className="shrink-0 px-1 text-[11px] text-[color:var(--kn-text-faint)] transition-colors hover:text-[color:var(--kn-accent)]"
      >
        ↺
      </button>
    ) : null;

  if (isBoolean) {
    const isOn = displayValue === "true";
    return (
      <div className="flex items-center gap-3 py-0.5">
        <div className="min-w-0 flex-1">
          <div className="flex items-center truncate text-xs text-[color:var(--kn-text-soft)]">
            {sectionTag}
            {label}
            {modifiedDot}
          </div>
          {description && (
            <div className="truncate text-[10px] text-[color:var(--kn-text-faint)]">
              {description}
            </div>
          )}
        </div>
        {resetButton}
        <button
          ref={registerRef}
          role="switch"
          aria-checked={isOn}
          onClick={() => onSave(key, isOn ? "false" : "true")}
          onKeyDown={(e) => onKeyDown(e, key, rowIdx, displayValue, "toggle")}
          className={`relative inline-flex h-5 w-9 shrink-0 items-center rounded-full border transition-colors ${
            isOn
              ? "border-[color:rgba(138,168,255,0.24)] bg-[color:var(--kn-accent)]"
              : "border-[color:var(--kn-border)] bg-white/[0.08]"
          } ${saving ? "opacity-60" : ""}`}
        >
          <span
            className={`inline-block h-3.5 w-3.5 transform rounded-full bg-white shadow transition-transform ${
              isOn ? "translate-x-[18px]" : "translate-x-[2px]"
            }`}
          />
        </button>
        <StatusBadge saving={saving} saved={saved} />
      </div>
    );
  }

  if (hasOptions) {
    const selectOptions = options.includes(displayValue) ? options : [displayValue, ...options];
    return (
      <div className="flex items-center gap-3">
        <label className="flex w-44 shrink-0 items-center truncate text-xs text-[color:var(--kn-text-muted)]">
          {sectionTag}
          {label}
          {modifiedDot}
        </label>
        <select
          ref={registerRef}
          value={displayValue}
          onChange={(e) => onSave(key, e.target.value)}
          onKeyDown={(e) => onKeyDown(e, key, rowIdx, displayValue, "select")}
          className={`kn-field flex-1 cursor-pointer px-2 py-1 text-sm ${saving ? "opacity-60" : ""}`}
        >
          {selectOptions.map((opt) => (
            <option key={opt} value={opt}>
              {opt}
            </option>
          ))}
        </select>
        {resetButton}
        <StatusBadge saving={saving} saved={saved} />
      </div>
    );
  }

  return (
    <div className="flex items-center gap-3">
      <label className="flex w-44 shrink-0 items-center truncate text-xs text-[color:var(--kn-text-muted)]">
        {sectionTag}
        {label}
        {modifiedDot}
      </label>
      <input
        ref={registerRef}
        value={displayValue}
        readOnly={isHotkey}
        type={isSecret ? "password" : "text"}
        onChange={isHotkey ? undefined : (e) => onChange(key, e.target.value)}
        onKeyDown={(e) => onKeyDown(e, key, rowIdx, displayValue, isHotkey ? "hotkey" : "text")}
        onBlur={isHotkey ? undefined : () => onBlur(key)}
        placeholder={
          isHotkey
            ? s.pressShortcut
            : isSecret
              ? secretIsSet
                ? s.secretSavedHint
                : s.secretNewHint
              : undefined
        }
        className={`kn-field flex-1 px-2 py-1 text-sm ${isHotkey ? "cursor-pointer" : ""} ${
          saving ? "opacity-60" : ""
        }`}
        spellCheck={false}
      />
      {isSecret && secretIsSet && !displayValue && (
        <span
          className="shrink-0 rounded bg-[color:var(--kn-success-wash)] px-1.5 py-0.5 text-[10px] text-[color:var(--kn-success)]"
          title="A value is stored in the OS keychain"
        >
          {s.secretSet}
        </span>
      )}
      {resetButton}
      <StatusBadge saving={saving} saved={saved} />
    </div>
  );
}
