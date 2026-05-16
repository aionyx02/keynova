import { useCallback, useRef, useState } from "react";

function readHistorySync(storageKey: string, cap: number): string[] {
  if (typeof window === "undefined") return [];
  try {
    const raw = window.localStorage.getItem(storageKey);
    if (!raw) return [];
    const parsed: unknown = JSON.parse(raw);
    if (Array.isArray(parsed) && parsed.every((v) => typeof v === "string")) {
      return parsed.slice(-cap);
    }
  } catch {
    // Corrupted slot — start fresh.
  }
  return [];
}

/**
 * Persistent ring buffer keyed by `localStorage` slot. Used by the AI panel
 * composer to give ↑/↓ history recall (chat / agent each have their own slot
 * per Phase 7a backlog spec).
 *
 * - `append(prompt)` pushes a new entry to the end, deduping consecutive
 *   identical prompts and capping at `cap`.
 * - `recall(direction)` returns the previous (`-1`) or next (`+1`) entry from
 *   the current cursor; `null` when the cursor walks past the ends.
 * - `resetCursor()` returns the cursor to "after the newest entry" — call it
 *   when the user starts typing fresh.
 */
export function useLocalHistory(storageKey: string, cap = 50) {
  const [history, setHistory] = useState<string[]>(() => readHistorySync(storageKey, cap));
  const cursorRef = useRef<number>(-1); // -1 means "below newest" (fresh input)

  const persist = useCallback(
    (next: string[]) => {
      try {
        window.localStorage.setItem(storageKey, JSON.stringify(next));
      } catch {
        // Quota / private mode — fall back to in-memory only.
      }
    },
    [storageKey],
  );

  const append = useCallback(
    (prompt: string) => {
      const trimmed = prompt.trim();
      if (!trimmed) return;
      setHistory((prev) => {
        if (prev[prev.length - 1] === trimmed) return prev; // dedupe consecutive
        const next = [...prev, trimmed].slice(-cap);
        persist(next);
        return next;
      });
      cursorRef.current = -1;
    },
    [cap, persist],
  );

  const recall = useCallback(
    (direction: -1 | 1): string | null => {
      if (history.length === 0) return null;
      let next = cursorRef.current + direction;
      if (next < 0) {
        // Walking older from "newest" → land on last entry.
        next = history.length - 1;
      } else if (next >= history.length) {
        cursorRef.current = -1;
        return ""; // signal: clear input
      }
      cursorRef.current = next;
      return history[next];
    },
    [history],
  );

  const resetCursor = useCallback(() => {
    cursorRef.current = -1;
  }, []);

  return { history, append, recall, resetCursor };
}
