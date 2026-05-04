import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";

export interface WorkspaceState {
  id: number;
  query: string;
  mode: string;
}

async function ipcDispatch<T>(route: string, payload?: Record<string, unknown>): Promise<T> {
  return invoke<T>("cmd_dispatch", { route, payload: payload ?? null });
}

export function useWorkspace() {
  const [current, setCurrent] = useState<WorkspaceState | null>(null);

  useEffect(() => {
    if (!window.__TAURI_INTERNALS__) return;
    ipcDispatch<WorkspaceState>("workspace.get_current")
      .then(setCurrent)
      .catch(() => {});

    const unlisten = listen<WorkspaceState>("workspace-switched", (event) => {
      setCurrent(event.payload);
    });
    return () => { unlisten.then((fn) => fn()); };
  }, []);

  const switchTo = useCallback(async (slot: number): Promise<WorkspaceState> => {
    const state = await ipcDispatch<WorkspaceState>("workspace.switch", { slot });
    setCurrent(state);
    return state;
  }, []);

  const saveCurrent = useCallback(async (query: string, mode: string) => {
    await ipcDispatch("workspace.save_current", { query, mode });
  }, []);

  return { current, switchTo, saveCurrent };
}