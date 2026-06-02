// Palette mode dispatch.
//
// Layered on top of `parseInputMode` (terminal/command sigils). When the
// upper parser puts us in `search`, this hook runs `parseCapabilityPrefix`
// and returns either `{ kind: "search" }` or `{ kind: "capability"; ... }`.
//
// Memoized: identical inputs yield the same object identity, so React
// reconciliation does not see capability swaps that did not actually
// happen. Callers can rely on `mode.kind === "capability"` and `mode.id`
// in dependency arrays.

import { useMemo } from "react";

import { parseInputMode } from "../../../hooks/useInputMode";
import { parseCapabilityPrefix, type CapabilityPrefixMatch } from "../utils/parseCapabilityPrefix";

export type PaletteMode = { kind: "search" } | ({ kind: "capability" } & CapabilityPrefixMatch);

const SEARCH_MODE: PaletteMode = { kind: "search" };

export function usePaletteMode(query: string): PaletteMode {
  return useMemo(() => {
    const inputMode = parseInputMode(query);
    if (inputMode.mode !== "search") return SEARCH_MODE;
    const match = parseCapabilityPrefix(query);
    if (!match) return SEARCH_MODE;
    return { kind: "capability", ...match };
  }, [query]);
}
