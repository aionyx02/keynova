// Transient UI hint state for clipboard / status messages.
//
// Owns two short-lived strings rendered in the palette footer area:
//   - `copiedPath`  — last path successfully copied via secondary-menu copy actions
//   - `copyHint`    — free-form status string (operation success / failure / progress)
//
// Both auto-clear after a TTL (default 1200ms) so the user sees a flash and
// then the next render returns to the normal footer content.

import { useCallback, useEffect, useRef, useState } from "react";

const DEFAULT_FLASH_MS = 1200;

export interface UseCopyHint {
  copiedPath: string | null;
  copyHint: string | null;
  /** Flash `path` as the last-copied path, auto-clear after `durationMs`. */
  flashCopiedPath: (path: string, durationMs?: number) => void;
  /** Flash a free-form message into `copyHint`, auto-clear after `durationMs`. */
  flashCopyHint: (message: string, durationMs?: number) => void;
  /** Clear both states immediately + cancel any pending auto-clear. */
  clear: () => void;
}

export function useCopyHint(): UseCopyHint {
  const [copiedPath, setCopiedPath] = useState<string | null>(null);
  const [copyHint, setCopyHint] = useState<string | null>(null);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Cancel any pending auto-clear on unmount so we don't poke React after
  // the component has gone away.
  useEffect(
    () => () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    },
    [],
  );

  const flashCopiedPath = useCallback((path: string, durationMs: number = DEFAULT_FLASH_MS) => {
    setCopiedPath(path);
    setCopyHint(null);
    if (timerRef.current) clearTimeout(timerRef.current);
    timerRef.current = setTimeout(() => setCopiedPath(null), durationMs);
  }, []);

  const flashCopyHint = useCallback((message: string, durationMs: number = DEFAULT_FLASH_MS) => {
    setCopiedPath(null);
    setCopyHint(message);
    if (timerRef.current) clearTimeout(timerRef.current);
    timerRef.current = setTimeout(() => setCopyHint(null), durationMs);
  }, []);

  const clear = useCallback(() => {
    if (timerRef.current) clearTimeout(timerRef.current);
    setCopiedPath(null);
    setCopyHint(null);
  }, []);

  return { copiedPath, copyHint, flashCopiedPath, flashCopyHint, clear };
}
