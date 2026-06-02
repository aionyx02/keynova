// Search backend metadata (active backend, index path, counts).
//
// Reads `search.backend` IPC once on mount and re-reads when a
// `config-reloaded` event reports that `search.backend` or
// `search.index_dir` changed. Surfaces a tiny chip in the palette header.

import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";

import type { DispatchFn } from "../../../context/IPCContext";
import { IPC } from "../../../ipc/routes";
import type { SearchBackendInfo } from "../../../ipc/types";

interface ConfigReloadedPayload {
  changed_keys: string[];
}

const WATCHED_KEYS: ReadonlyArray<string> = ["search.backend", "search.index_dir"];

export interface UseSearchBackendDeps {
  dispatch: DispatchFn;
}

export interface UseSearchBackend {
  searchBackend: SearchBackendInfo | null;
}

export function useSearchBackend({ dispatch }: UseSearchBackendDeps): UseSearchBackend {
  const [searchBackend, setSearchBackend] = useState<SearchBackendInfo | null>(null);

  useEffect(() => {
    if (!window.__TAURI_INTERNALS__) return;

    async function refresh() {
      try {
        const info = await dispatch<SearchBackendInfo>(IPC.SEARCH_BACKEND);
        setSearchBackend(info);
      } catch {
        setSearchBackend(null);
      }
    }

    void refresh();
    const unlisten = listen<ConfigReloadedPayload>("config-reloaded", (event) => {
      const keys = event.payload.changed_keys;
      if (keys.length === 0 || WATCHED_KEYS.some((k) => keys.includes(k))) {
        void refresh();
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
    // dispatch is intentionally omitted — useIPC returns a fresh wrapper each render.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return { searchBackend };
}
