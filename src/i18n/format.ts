/**
 * Minimal placeholder interpolation for i18n strings. Locale values stay plain
 * strings (so `I18nKeys` keeps its simple shape); callers fill `{name}`-style
 * tokens at render time: `fmt(t.model.activeNow, { name })`.
 */
export function fmt(template: string, vars: Record<string, string | number>): string {
  return template.replace(/\{(\w+)\}/g, (_, key) =>
    key in vars ? String(vars[key]) : `{${key}}`,
  );
}
