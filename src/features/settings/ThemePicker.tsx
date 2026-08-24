// Visual picker for `launcher.theme`.
//
// A colour scheme cannot be chosen from a <select>: the point is seeing it.
// Each card wraps its preview in `data-theme`, which rebinds the whole variable
// set for that subtree (see the selector note in index.css), so the previews
// read their colours from the stylesheet rather than repeating the palette
// here. Adding a theme means adding a CSS block and a name — nothing in this
// file changes.

import type React from "react";

import { THEMES, type Theme } from "../../shared/theme";
import { useI18n } from "../../i18n/useI18n";

interface Props {
  value: Theme;
  saving: boolean;
  registerRef: (el: HTMLElement | null) => void;
  onSelect: (theme: Theme) => void;
  /** Falls through to the panel's row navigation for keys not consumed here. */
  onKeyDown: (e: React.KeyboardEvent<HTMLElement>) => void;
}

/** A palette in miniature: input row, a selected result, two plain ones. */
function Preview() {
  return (
    <div
      style={{
        height: "62px",
        overflow: "hidden",
        borderRadius: "4px",
        background: "var(--kn-panel-bg)",
        boxShadow: "inset 0 0 0 1px var(--kn-ring)",
      }}
    >
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: "4px",
          height: "17px",
          padding: "0 6px",
        }}
      >
        <span
          style={{
            width: "5px",
            height: "5px",
            flexShrink: 0,
            borderRadius: "2px",
            background: "var(--kn-text-faint)",
          }}
        />
        <span style={{ width: "1.5px", height: "8px", background: "var(--kn-accent)" }} />
        <span
          style={{
            width: "26px",
            height: "3px",
            borderRadius: "2px",
            background: "var(--kn-text-faint)",
          }}
        />
      </div>
      <div style={{ height: "1px", background: "var(--kn-border)" }} />
      <div style={{ display: "flex", flexDirection: "column", gap: "2px", padding: "3px" }}>
        {[0, 1, 2].map((i) => (
          <div
            key={i}
            style={{
              display: "flex",
              alignItems: "center",
              gap: "4px",
              height: "12px",
              padding: "0 4px",
              borderRadius: "4px",
              background: i === 0 ? "var(--kn-selected)" : "transparent",
            }}
          >
            <span
              style={{
                width: "5px",
                height: "5px",
                flexShrink: 0,
                borderRadius: "2px",
                background: i === 0 ? "var(--kn-text-soft)" : "var(--kn-text-faint)",
              }}
            />
            <span
              style={{
                width: i === 0 ? "46px" : i === 1 ? "34px" : "40px",
                height: "3px",
                borderRadius: "2px",
                background: i === 0 ? "var(--kn-text)" : "var(--kn-text-muted)",
              }}
            />
          </div>
        ))}
      </div>
    </div>
  );
}

export function ThemePicker({ value, saving, registerRef, onSelect, onKeyDown }: Props) {
  const s = useI18n().settings;

  function handleKeyDown(e: React.KeyboardEvent<HTMLElement>) {
    // Left/Right move between themes here; everywhere else in the panel they
    // switch section, so they must be consumed before the shared handler sees
    // them. Up/Down and the rest still fall through to row navigation.
    if (e.key === "ArrowLeft" || e.key === "ArrowRight") {
      e.preventDefault();
      const at = THEMES.indexOf(value);
      const step = e.key === "ArrowRight" ? 1 : THEMES.length - 1;
      onSelect(THEMES[(at + step) % THEMES.length]);
      return;
    }
    onKeyDown(e);
  }

  return (
    <div
      role="radiogroup"
      aria-label={s.themeLabel}
      className={`grid grid-cols-3 gap-2 ${saving ? "opacity-60" : ""}`}
    >
      {THEMES.map((theme, index) => {
        const isSelected = theme === value;
        return (
          <button
            key={theme}
            // Only the selected card is tabbable, so Tab leaves the group
            // instead of walking every option — standard radiogroup behaviour.
            ref={isSelected ? registerRef : undefined}
            type="button"
            role="radio"
            aria-checked={isSelected}
            tabIndex={isSelected ? 0 : -1}
            onClick={() => onSelect(theme)}
            onKeyDown={handleKeyDown}
            className="cursor-pointer rounded-[10px] p-1.5 text-left transition-colors"
            style={{
              background: "var(--kn-panel-bg-soft)",
              boxShadow: isSelected ? "inset 0 0 0 1.5px var(--kn-accent)" : "none",
            }}
          >
            <div data-theme={theme}>
              <Preview />
            </div>
            <div className="mt-2 flex items-center justify-between gap-2">
              <span
                className="truncate text-xs"
                style={{
                  color: isSelected ? "var(--kn-text)" : "var(--kn-text-muted)",
                  fontWeight: isSelected ? 500 : 400,
                }}
              >
                {s.themeNames[theme] ?? theme}
              </span>
              {isSelected && (
                <svg
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth={2}
                  aria-hidden="true"
                  className="h-3.5 w-3.5 shrink-0"
                  style={{ color: "var(--kn-accent)" }}
                >
                  <path d="m5 12.5 4.5 4.5L19 7.5" strokeLinecap="round" strokeLinejoin="round" />
                </svg>
              )}
            </div>
            <span className="sr-only">{index + 1}</span>
          </button>
        );
      })}
    </div>
  );
}
