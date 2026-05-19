import type { SearchResult } from "../types/search";

export function searchResultKey(result: SearchResult): string {
  return `${result.source ?? result.kind}:${result.path}`;
}

// Per-source caps mirror the backend sort_balanced_truncate constants.
export const SOURCE_QUOTAS: Partial<Record<string, number>> = {
  app: 8,
  command: 8,
  note: 8,
  history: 12,
  model: 6,
};

export function searchSourceOrder(result: SearchResult): number {
  switch (result.kind) {
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

export function sortSearchResults(results: SearchResult[]): SearchResult[] {
  return [...results].sort((left, right) => {
    if (right.score !== left.score) return right.score - left.score;
    const sourceOrder = searchSourceOrder(left) - searchSourceOrder(right);
    if (sourceOrder !== 0) return sourceOrder;
    return (left.title ?? left.name).localeCompare(right.title ?? right.name);
  });
}

export function applySourceQuotas(sorted: SearchResult[], limit: number): SearchResult[] {
  const counts: Record<string, number> = {};
  const out: SearchResult[] = [];
  for (const item of sorted) {
    const key = item.source ?? item.kind;
    const quota = SOURCE_QUOTAS[key];
    const n = counts[key] ?? 0;
    if (quota !== undefined && n >= quota) continue;
    counts[key] = n + 1;
    out.push(item);
    if (out.length >= limit) break;
  }
  return out;
}

export function mergeSearchResults(
  existing: SearchResult[],
  incoming: SearchResult[],
  limit: number,
): SearchResult[] {
  const seen = new Set(existing.map(searchResultKey));
  const merged = [...existing];
  for (const item of incoming) {
    const key = searchResultKey(item);
    if (seen.has(key)) continue;
    seen.add(key);
    merged.push(item);
  }
  return applySourceQuotas(sortSearchResults(merged), limit);
}