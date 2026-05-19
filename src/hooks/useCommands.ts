import { useCallback, useEffect, useState } from "react";
import { useIPC } from "./useIPC";
import { IPC } from "../ipc/routes";
import type { TerminalLaunchSpec } from "../types/terminal";

export interface CommandMeta {
  name: string;
  description: string;
  args_hint?: string;
}

export type CommandUiType =
  | { type: "Inline" }
  | { type: "Panel"; value: string }
  | { type: "Terminal"; value: TerminalLaunchSpec };

export interface BuiltinCommandResult {
  text: string;
  ui_type: CommandUiType;
}

export function useCommands() {
  const { dispatch } = useIPC();
  const [all, setAll] = useState<CommandMeta[]>([]);

  useEffect(() => {
    if (!window.__TAURI_INTERNALS__) return;
    dispatch<CommandMeta[]>(IPC.CMD_LIST).then(setAll).catch(() => {});
  }, [dispatch]);

  const filtered = useCallback(
    (input: string): CommandMeta[] => {
      const q = input.toLowerCase().trim();
      if (!q) return all;
      return all.filter((c) => c.name.includes(q) || c.description.includes(q));
    },
    [all],
  );

  const runCommand = useCallback(
    (name: string, args = ""): Promise<BuiltinCommandResult> => {
      return dispatch<BuiltinCommandResult>(IPC.CMD_RUN, { name, args });
    },
    [dispatch],
  );

  const suggestArgs = useCallback(
    (name: string, partial: string): Promise<string[]> => {
      return dispatch<string[]>(IPC.CMD_SUGGEST_ARGS, { name, partial });
    },
    [dispatch],
  );

  return { all, filtered, runCommand, suggestArgs };
}