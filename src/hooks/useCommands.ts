import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export interface CommandMeta {
  name: string;
  description: string;
  args_hint?: string;
}

export interface CommandUiType {
  type: "Inline" | "Panel";
  value?: string; // panel name when type === "Panel"
}

export interface BuiltinCommandResult {
  text: string;
  ui_type: CommandUiType;
}

async function ipcDispatch<T>(route: string, payload?: Record<string, unknown>): Promise<T> {
  return invoke<T>("cmd_dispatch", { route, payload: payload ?? null });
}

export function useCommands() {
  const [all, setAll] = useState<CommandMeta[]>([]);

  useEffect(() => {
    if (!window.__TAURI_INTERNALS__) return;
    ipcDispatch<CommandMeta[]>("cmd.list")
      .then(setAll)
      .catch(() => {});
  }, []);

  const filtered = useCallback(
    (input: string): CommandMeta[] => {
      const q = input.toLowerCase().trim();
      if (!q) return all;
      return all.filter((c) => c.name.includes(q) || c.description.includes(q));
    },
    [all],
  );

  const runCommand = useCallback(
    async (name: string, args = ""): Promise<BuiltinCommandResult> => {
      return ipcDispatch<BuiltinCommandResult>("cmd.run", { name, args });
    },
    [],
  );

  const suggestArgs = useCallback(
    async (name: string, partial: string): Promise<string[]> => {
      return ipcDispatch<string[]>("cmd.suggest_args", { name, partial });
    },
    [],
  );

  return { all, filtered, runCommand, suggestArgs };
}