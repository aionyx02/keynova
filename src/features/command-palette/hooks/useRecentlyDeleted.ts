// Kill set for recently deleted, moved, or renamed paths.
//
// Why this exists: Windows Everything keeps Recycle Bin entries indexed for
// a while after a delete, so a just-trashed file can resurface in a streaming
// search chunk between the user's destructive action and Everything's
// reindex. This hook holds a render-time kill set with a 30 s TTL: every
// chunk path is checked against the set before reaching `visibleResults`,
// so it does not matter which code path (initial debounce response,
// stream chunk, merge) reintroduces the trashed path — it cannot render.
//
// TTL = 30 s so that if the user restores the file from Recycle Bin within
// the same launcher session, it becomes searchable again without needing a
// workspace switch.

import { useCallback, useState } from "react";

export const RECENTLY_DELETED_TTL_MS = 30_000;

export interface UseRecentlyDeleted {
  /** Record a path as recently deleted; subsequent isDeleted(path) returns true within TTL. */
  markDeleted: (path: string) => void;
  /** Drop all kill-set entries (used on query change / workspace switch). */
  clear: () => void;
  /** True if path is in the kill set and TTL has not expired. */
  isDeleted: (path: string, now?: number) => boolean;
}

export function useRecentlyDeleted(): UseRecentlyDeleted {
  const [recentlyDeleted, setRecentlyDeleted] = useState<Map<string, number>>(() => new Map());

  const markDeleted = useCallback((path: string) => {
    setRecentlyDeleted((prev) => {
      const next = new Map(prev);
      next.set(path, Date.now());
      return next;
    });
  }, []);

  const clear = useCallback(() => {
    setRecentlyDeleted(new Map());
  }, []);

  const isDeleted = useCallback(
    (path: string, now: number = Date.now()) => {
      const ts = recentlyDeleted.get(path);
      return ts !== undefined && ts > now - RECENTLY_DELETED_TTL_MS;
    },
    [recentlyDeleted],
  );

  return { markDeleted, clear, isDeleted };
}
