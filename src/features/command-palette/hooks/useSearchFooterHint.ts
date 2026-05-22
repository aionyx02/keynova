// REF.2.P5 — Footer hint string for the search results card.
//
// Priority chain (first non-empty wins):
//   1. `copyHint` (transient, e.g. "SHA-256 copied: …"),
//   2. `copiedPath`        ("Copied path: <path>"),
//   3. provider timeout    ("Timed out: <comma list>"),
//   4. file-search diagnostic (fallback reason / display-limit truncation),
//   5. selected file's preview text,
//   6. selected file's size_bytes,
//   7. default keyboard hint.
//
// Extracted because the inline ternary in CommandPalette had grown to
// 7 levels deep — easier to maintain as a regular function.

import type { SearchChunkDiagnostics, SearchMetadata } from "../../../types/search";

interface Deps {
  copyHint: string | null;
  copiedPath: string | null;
  timedOutProviders: string[];
  fileDiagnostics: SearchChunkDiagnostics | null;
  selectedMetadata: SearchMetadata | null | undefined;
}

export function useSearchFooterHint({
  copyHint,
  copiedPath,
  timedOutProviders,
  fileDiagnostics,
  selectedMetadata,
}: Deps): string {
  if (copyHint) return copyHint;
  if (copiedPath) return `Copied path: ${copiedPath}`;
  if (timedOutProviders.length > 0 && !fileDiagnostics) {
    return `Timed out: ${timedOutProviders.join(", ")}`;
  }

  if (fileDiagnostics) {
    if (fileDiagnostics.timed_out) {
      return "File search: provider timed out after 800ms";
    }
    if (fileDiagnostics.fallback_reason) {
      return `File search: ${fileDiagnostics.fallback_reason}`;
    }
    const hidden = fileDiagnostics.pre_balance_count - fileDiagnostics.returned_count;
    if (hidden > 0) {
      return `File search: ${fileDiagnostics.returned_count} shown, ${hidden} hidden by display limit`;
    }
  }

  if (selectedMetadata?.preview) return selectedMetadata.preview;
  if (selectedMetadata?.size_bytes !== undefined) {
    return `${selectedMetadata.size_bytes.toLocaleString()} bytes`;
  }
  return "↑↓ 選擇";
}
