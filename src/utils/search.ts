import type { ResultKind, SearchResult } from "../types/search";
import type { UnifiedResult } from "../types/unified-result";

// REF.6.A — palette wire format is `UnifiedResult`. Helpers below operate on
// the canonical shape. Legacy `SearchResult` callers convert via
// [`unifiedToLegacy`] until REF.6.B migrates them.

export function unifiedResultKey(result: UnifiedResult): string {
  // Stable per generation: backend sets `id = item_ref.id` in the shim and
  // uses the path for file sources only.
  return result.id;
}

// Per-source caps mirror the backend sort_balanced_truncate constants.
export const SOURCE_QUOTAS: Partial<Record<string, number>> = {
  app: 8,
  command: 8,
  note: 8,
  history: 12,
  model: 6,
};

/** Derive the legacy `ResultKind` slot for a UnifiedResult, when applicable. */
export function unifiedKindOf(result: UnifiedResult): ResultKind | null {
  if (result.source.type === "file") return result.source.kind;
  return null;
}

/** Bucket key used for source-quota balancing. */
function unifiedBucketKey(result: UnifiedResult): string {
  if (result.source.type === "file") return result.source.kind;
  if (result.source.type === "builtin_command") return "command";
  return result.source.name;
}

function unifiedSourceOrder(result: UnifiedResult): number {
  const kind = unifiedKindOf(result);
  switch (kind) {
    case "app":     return 0;
    case "command": return 1;
    case "folder":  return 2;
    case "file":    return 3;
    case "note":    return 4;
    case "history": return 5;
    case "model":   return 6;
    default:        return 7;
  }
}

export function sortUnifiedResults(results: UnifiedResult[]): UnifiedResult[] {
  return [...results].sort((left, right) => {
    const leftScore = left.rank?.score ?? 0;
    const rightScore = right.rank?.score ?? 0;
    if (rightScore !== leftScore) return rightScore - leftScore;
    const sourceOrder = unifiedSourceOrder(left) - unifiedSourceOrder(right);
    if (sourceOrder !== 0) return sourceOrder;
    return left.title.localeCompare(right.title);
  });
}

export function applySourceQuotas(
  sorted: UnifiedResult[],
  limit: number,
): UnifiedResult[] {
  const counts: Record<string, number> = {};
  const out: UnifiedResult[] = [];
  for (const item of sorted) {
    const key = unifiedBucketKey(item);
    const quota = SOURCE_QUOTAS[key];
    const n = counts[key] ?? 0;
    if (quota !== undefined && n >= quota) continue;
    counts[key] = n + 1;
    out.push(item);
    if (out.length >= limit) break;
  }
  return out;
}

export function mergeUnifiedResults(
  existing: UnifiedResult[],
  incoming: UnifiedResult[],
  limit: number,
): UnifiedResult[] {
  const seen = new Set(existing.map(unifiedResultKey));
  const merged = [...existing];
  for (const item of incoming) {
    const key = unifiedResultKey(item);
    if (seen.has(key)) continue;
    seen.add(key);
    merged.push(item);
  }
  return applySourceQuotas(sortUnifiedResults(merged), limit);
}

/**
 * REF.6.A bridge — adapt a UnifiedResult into the legacy `SearchResult` shape
 * so existing hooks (`useFileActions`, `useDerivedView`, `useKeyboardNav`,
 * `useFilePreview`, etc.) keep working without per-file migration. The
 * canonical shape stays as `UnifiedResult`; this is a one-way derived view.
 */
export function unifiedToLegacy(result: UnifiedResult): SearchResult {
  const primaryAction = result.actions.find((a) => a.primary) ?? result.actions[0];
  let kind: ResultKind = "file";
  let path = result.subtitle;
  let source = "file";
  if (result.source.type === "file") {
    kind = result.source.kind;
    path = result.source.path;
    source = kind === "app" ? "app"
      : kind === "command" ? "command"
      : kind === "note" ? "note"
      : kind === "history" ? "history"
      : kind === "model" ? "model"
      : "file";
  } else if (result.source.type === "builtin_command") {
    kind = "command";
    source = "command";
  }
  return {
    item_ref: primaryAction?.action_ref,
    title: result.title,
    subtitle: result.subtitle,
    source,
    icon_key: result.icon_key ?? null,
    primary_action: primaryAction?.action_ref,
    primary_action_label: primaryAction?.label,
    secondary_action_count: result.source_metadata?.secondary_action_count ?? 0,
    kind,
    name: result.title,
    path,
    score: result.rank?.score ?? 0,
    score_breakdown: result.rank?.breakdown,
    unified_id: result.id,
  };
}