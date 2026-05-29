// REF.6.B / REF.6.D — Capability-mode prefix parser.
//
// Layered on top of `parseInputMode` (which classifies `> ...` as terminal
// and `/ ...` as command). When `parseInputMode` returns `search`, this
// parser runs against the raw query to detect a leading capability keyword.
//
// Wired prefixes: `explain`, `summarize`, `cmd`, `fix`, `next`.
//
// Rules:
// - Match is case-insensitive on the keyword, body preserved as typed
//   (then trimmed).
// - `explain` / `summarize` / `cmd` / `fix` require keyword + at least one
//   space + non-empty body. Bare `explain` without trailing space is NOT a
//   match (user might be searching for the word "explain").
// - `next` is the only zero-arg capability; `next` and `next   ` both match.
// - Whitespace-only bodies return null (don't fire a capability call for
//   nothing).
// - Leading whitespace on the query is allowed.

export type CapabilityTextPrefixId = "explain" | "summarize" | "cmd" | "fix";
export type CapabilityPrefixId = CapabilityTextPrefixId | "next";

export interface CapabilityTextArgs {
  text: string;
}

export type CapabilityPrefixMatch =
  | { id: CapabilityTextPrefixId; args: CapabilityTextArgs }
  | { id: "next"; args: Record<string, never> };

interface PrefixSpec {
  keyword: string;
  id: CapabilityTextPrefixId;
}

// Order matters only for documentation; longest-first is unnecessary because
// no prefix is a substring of another.
const PREFIXES: ReadonlyArray<PrefixSpec> = [
  { keyword: "explain", id: "explain" },
  { keyword: "summarize", id: "summarize" },
  { keyword: "cmd", id: "cmd" },
  { keyword: "fix", id: "fix" },
];

export function parseCapabilityPrefix(
  query: string,
): CapabilityPrefixMatch | null {
  if (!query) return null;
  // Strip leading whitespace once; we test the rest against each prefix.
  const lstripped = query.replace(/^\s+/, "");
  const nextRest = lstripped.slice("next".length);
  if (
    lstripped.length >= "next".length &&
    lstripped.slice(0, "next".length).toLowerCase() === "next" &&
    /^\s*$/.test(nextRest)
  ) {
    return { id: "next", args: {} };
  }
  for (const { keyword, id } of PREFIXES) {
    if (lstripped.length < keyword.length + 1) continue;
    if (lstripped.slice(0, keyword.length).toLowerCase() !== keyword) continue;
    if (lstripped.charAt(keyword.length) !== " ") continue;
    const body = lstripped.slice(keyword.length + 1).trim();
    if (!body) return null;
    return { id, args: { text: body } };
  }
  return null;
}
