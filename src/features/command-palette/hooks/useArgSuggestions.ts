// REF.2.P5 — Args-phase suggestions hook.
//
// When the user types `/cmd ` (a known command followed by a space) the
// palette enters "args phase": the command is locked and the trailing text
// is treated as arguments for that command. This hook owns the derivation
// of the locked command (`exactCmd`), the boolean phase flag, and the
// debounced fetch of argument suggestions from `suggestArgs`.
//
// Clearing on phase exit is the caller's responsibility (CommandPalette
// resets argSuggestions inside its `handleQueryChange` because doing it
// inside an effect would race with synchronous setState during typing).

import { useEffect, useRef, useState } from "react";

import type { CommandMeta } from "../../../hooks/useCommands";

interface Deps {
  mode: "search" | "command" | "terminal";
  cmdName: string;
  cmdArgs: string;
  spaceIdx: number;
  all: CommandMeta[];
  suggestArgs: (name: string, args: string) => Promise<string[]>;
}

interface UseArgSuggestions {
  exactCmd: CommandMeta | null;
  isArgsPhase: boolean;
  argSuggestions: string[];
  setArgSuggestions: React.Dispatch<React.SetStateAction<string[]>>;
  selectedArg: number;
  setSelectedArg: React.Dispatch<React.SetStateAction<number>>;
}

export function useArgSuggestions({
  mode,
  cmdName,
  cmdArgs,
  spaceIdx,
  all,
  suggestArgs,
}: Deps): UseArgSuggestions {
  const [argSuggestions, setArgSuggestions] = useState<string[]>([]);
  const [selectedArg, setSelectedArg] = useState(0);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const exactCmd =
    mode === "command" && spaceIdx !== -1
      ? (all.find((c) => c.name === cmdName) ?? null)
      : null;
  const isArgsPhase = exactCmd !== null;

  useEffect(() => {
    if (!isArgsPhase || !exactCmd) return;
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(() => {
      suggestArgs(exactCmd.name, cmdArgs)
        .then((results) => {
          setArgSuggestions(results);
          setSelectedArg(0);
        })
        .catch(() => {});
    }, 150);
    return () => {
      if (debounceRef.current) clearTimeout(debounceRef.current);
    };
    // suggestArgs is intentionally omitted — useCommands returns a fresh wrapper each render.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isArgsPhase, exactCmd?.name, cmdArgs]);

  return {
    exactCmd,
    isArgsPhase,
    argSuggestions,
    setArgSuggestions,
    selectedArg,
    setSelectedArg,
  };
}
