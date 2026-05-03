import { useCallback } from "react";
import { useIPC } from "./useIPC";
import { useEvents } from "./useEvents";

interface TerminalOutputEvent {
  id: string;
  output: string;
}

interface OpenTerminalResponse {
  id: string;
  initial_output: string;
}

export function useTerminal(onOutput: (id: string, data: string) => void) {
  const { dispatch } = useIPC();

  const handleOutput = useCallback(
    (event: TerminalOutputEvent) => onOutput(event.id, event.output),
    [onOutput],
  );

  useEvents<TerminalOutputEvent>("terminal-output", handleOutput);

  async function open(): Promise<string> {
    const resp = await dispatch<OpenTerminalResponse>("terminal.open");
    return resp.id;
  }

  async function send(id: string, input: string): Promise<void> {
    return dispatch("terminal.send", { id, input });
  }

  async function close(id: string): Promise<void> {
    return dispatch("terminal.close", { id });
  }

  async function resize(id: string, rows: number, cols: number): Promise<void> {
    return dispatch("terminal.resize", { id, rows, cols });
  }

  return { open, send, close, resize };
}
