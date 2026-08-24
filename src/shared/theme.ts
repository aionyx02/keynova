// Palette appearance, applied as `data-theme` on the root element.
//
// index.css defines one variable set per theme and nothing else reads a raw
// colour, so switching is this attribute and nothing more. The default carries
// NO attribute — `:root` already is the default theme — which keeps the common
// case free of a DOM write and means an unreadable setting degrades to the
// theme the stylesheet was authored against.
//
// This lives in `shared/` rather than inside the command palette because the
// root element is not owned by any component, and because settings is planned
// to move into its own window (docs/state.md, Planned) — at which point both
// windows apply the theme through this same function.

export const THEMES = ["warm", "frosted", "ink"] as const;

export type Theme = (typeof THEMES)[number];

export const DEFAULT_THEME: Theme = "warm";

/** Narrows an arbitrary setting value to a theme this build knows about. */
export function isTheme(value: unknown): value is Theme {
  return typeof value === "string" && (THEMES as ReadonlyArray<string>).includes(value);
}

/**
 * Resolves any stored setting value to a usable theme.
 *
 * A value written by a newer build, a hand-edited config, or nothing at all
 * all land on the default rather than on a theme with no variable set behind
 * it, which would render an unstyled palette.
 */
export function resolveTheme(value: string | undefined): Theme {
  return isTheme(value) ? value : DEFAULT_THEME;
}

/** Writes the theme onto the document root. Safe to call repeatedly. */
export function applyTheme(theme: Theme, root: HTMLElement = document.documentElement): void {
  if (theme === DEFAULT_THEME) {
    delete root.dataset.theme;
    return;
  }
  root.dataset.theme = theme;
}
