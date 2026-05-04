import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";

export interface ClipboardEntry {
  id: string;
  content: string;
  content_type: string;
  timestamp: number;
  pinned: boolean;
}

async function ipcDispatch<T>(route: string, payload?: Record<string, unknown>): Promise<T> {
  return invoke<T>("cmd_dispatch", { route, payload: payload ?? null });
}

export function useHistory() {
  const [entries, setEntries] = useState<ClipboardEntry[]>([]);

  const refresh = useCallback(async () => {
    if (!window.__TAURI_INTERNALS__) return;
    try {
      const list = await ipcDispatch<ClipboardEntry[]>("history.list");
      setEntries(list);
    } catch {
      setEntries([]);
    }
  }, []);

  useEffect(() => {
    if (!window.__TAURI_INTERNALS__) return;
    ipcDispatch<ClipboardEntry[]>("history.list")
      .then(setEntries)
      .catch(() => setEntries([]));
    const unlisten = listen("history-updated", () => {
      ipcDispatch<ClipboardEntry[]>("history.list")
        .then(setEntries)
        .catch(() => {});
    });
    return () => { unlisten.then((fn) => fn()); };
  }, []);

  const search = useCallback(async (q: string): Promise<ClipboardEntry[]> => {
    return ipcDispatch<ClipboardEntry[]>("history.search", { q });
  }, []);

  const deleteEntry = useCallback(async (id: string) => {
    await ipcDispatch("history.delete", { id });
    await refresh();
  }, [refresh]);

  const pinEntry = useCallback(async (id: string, pinned: boolean) => {
    await ipcDispatch("history.pin", { id, pinned });
    await refresh();
  }, [refresh]);

  const clearAll = useCallback(async () => {
    await ipcDispatch("history.clear");
    await refresh();
  }, [refresh]);

  return { entries, refresh, search, deleteEntry, pinEntry, clearAll };
}