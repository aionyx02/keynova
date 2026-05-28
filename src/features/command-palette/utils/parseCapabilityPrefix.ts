// REF.6.B — Capability-mode prefix parser.
//
// Layered on top of `parseInputMode` (which classifies `> ...` as terminal
// and `/ ...` as command). When `parseInputMode` returns `search`, this
// parser runs against the raw query to detect a leading capability keyword.
//
// Wired prefixes in this batch: `explain`, `summarize`. Future batches add
// `cmd`, `fix`, `next`; the parser intentionally returns null for those
// today so an early adopter typing `fix foo` still gets normal search
// results until REF.6.D lands.
//
// Rules:
// - Match is case-insensitive on the keyword, body preserved as typed
//   (then trimmed).
// - Requires keyword + at least one space + non-empty body. Bare `explain`
//   without trailing space is NOT a match (user might be searching for the
//   word "explain").
// - Whitespace-only body returns null (don't fire a capability call for
//   nothing).
// - Leading whitespace on the query is allowed.

export type CapabilityPrefixId = "explain" | "summarize";

export interface CapabilityPrefixMatch {
  id: CapabilityPrefixId;
  args: { text: string };
}

interface PrefixSpec {
  keyword: string;
  id: CapabilityPrefixId;
}

// Order matters only for documentation; longest-first is unnecessary because
// no prefix is a substring of another.
const PREFIXES: ReadonlyArray<PrefixSpec> = [
  { keyword: "explain", id: "explain" },
  { keyword: "summarize", id: "summarize" },
];

export function parseCapabilityPrefix(
  query: string,
): CapabilityPrefixMatch | null {
  if (!query) return null;
  // Strip leading whitespace once; we test the rest against each prefix.
  const lstripped = query.replace(/^\s+/, "");
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
