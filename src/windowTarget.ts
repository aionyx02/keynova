// Which window this webview is.
//
// Both windows are served from the same `index.html`: one Vite entry, one
// bundle graph, no `rollupOptions.input`. They split at runtime on the query
// string, and `main.tsx` dynamic-imports one of two trees from that decision —
// so the settings webview never downloads the palette, xterm, or markdown
// chunks it has no use for.

export type WindowTarget = "launcher" | "settings";

/** The launcher carries no query at all, so it is also the fallback. */
export const DEFAULT_WINDOW_TARGET: WindowTarget = "launcher";

/**
 * Resolves `?window=` to a window this build knows how to render.
 *
 * Anything unrecognised lands on the launcher rather than on a blank root: an
 * unknown value means the URL is older or newer than this bundle, and a working
 * launcher is a better failure than an empty window with no way out.
 */
export function resolveWindowTarget(search: string): WindowTarget {
  return new URLSearchParams(search).get("window") === "settings"
    ? "settings"
    : DEFAULT_WINDOW_TARGET;
}
