// Footer hint string for the search results card.
//
// Priority chain (first non-empty wins):
//   1. `copyHint` (transient, e.g. "SHA-256 copied: ..."),
//   2. `copiedPath`,
//   3. provider timeout,
//   4. file-search diagnostic (fallback reason / display-limit truncation),
//   5. selected file's preview text,
//   6. selected file's size_bytes,
//   7. default keyboard hint.
//
// Extracted because the inline ternary in CommandPalette had grown to
// 7 levels deep, easier to maintain as a regular function.

import { fmt } from "../../../i18n/format";
import { useI18n } from "../../../i18n/useI18n";
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
  const p = useI18n().palette;

  if (copyHint) return copyHint;
  if (copiedPath) return fmt(p.copiedPath, { path: copiedPath });
  if (timedOutProviders.length > 0 && !fileDiagnostics) {
    return fmt(p.timedOut, { providers: timedOutProviders.join(", ") });
  }

  if (fileDiagnostics) {
    if (fileDiagnostics.timed_out) {
      return p.fileTimedOut;
    }
    if (fileDiagnostics.indexing) {
      return p.fileIndexing;
    }
    if (fileDiagnostics.fallback_reason) {
      return fmt(p.fileFallback, { reason: fileDiagnostics.fallback_reason });
    }
    const hidden = fileDiagnostics.pre_balance_count - fileDiagnostics.returned_count;
    if (hidden > 0) {
      return fmt(p.fileDisplayLimit, {
        shown: fileDiagnostics.returned_count,
        hidden,
      });
    }
  }

  if (selectedMetadata?.preview) return selectedMetadata.preview;
  if (selectedMetadata?.size_bytes !== undefined) {
    return fmt(p.bytes, { count: selectedMetadata.size_bytes.toLocaleString() });
  }
  return `↑/↓ ${p.navigate}`;
}
