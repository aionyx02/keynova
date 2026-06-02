import { useEffect, useRef, useState } from "react";
import { useIPC } from "./useIPC";
import { IPC } from "../ipc/routes";
import type { FilePreviewResult, SearchResult } from "../types/search";

const PREVIEW_KINDS = new Set(["file", "folder", "note"]);
const DEBOUNCE_MS = 80;
const CACHE_CAP = 64;

/**
 * Fetches `file.preview` for the focused result, debounced so
 * rapid ↑/↓ collapses to one IPC dispatch. Caches by path (LRU, cap 64) and
 * skips non-previewable kinds (app / command / history / model).
 *
 * `loading` is derived at the call site (`isPreviewable(result) && !previewByPath[path]`)
 * — no setState in effect.
 */
export function useFilePreview(results: SearchResult[], selected: number) {
  const { dispatch } = useIPC();
  const [previewByPath, setPreviewByPath] = useState<Record<string, FilePreviewResult>>({});
  // Track insertion order for LRU eviction without growing the map unbounded.
  const orderRef = useRef<string[]>([]);

  useEffect(() => {
    if (typeof window === "undefined" || !window.__TAURI_INTERNALS__) return;
    const result = results[selected];
    if (!result) return;
    if (!PREVIEW_KINDS.has(result.kind)) return;
    if (previewByPath[result.path]) return;

    let cancelled = false;
    const timer = setTimeout(() => {
      dispatch<FilePreviewResult>(IPC.FILE_PREVIEW, { path: result.path })
        .then((preview) => {
          if (cancelled) return;
          // Key the cache by the requested result path; image previews no longer
          // echo a `path` field (security wave B #1), and keying by request is
          // correct regardless.
          const key = result.path;
          setPreviewByPath((prev) => {
            // Skip if a racing fetch already filled it.
            if (prev[key]) return prev;
            const next = { ...prev, [key]: preview };
            orderRef.current.push(key);
            while (orderRef.current.length > CACHE_CAP) {
              const evicted = orderRef.current.shift();
              if (evicted && evicted !== key) {
                delete next[evicted];
              }
            }
            return next;
          });
        })
        .catch(() => {
          // Swallow; PreviewPane will fall back to "no preview available".
        });
    }, DEBOUNCE_MS);

    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, [dispatch, results, selected, previewByPath]);

  return { previewByPath };
}

/** Returns true if the result's kind is one for which we attempt a preview. */
export function isPreviewable(result: SearchResult | null): boolean {
  return !!result && PREVIEW_KINDS.has(result.kind);
}
