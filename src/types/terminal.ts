export type TerminalStatus = "Running" | "Exited";

export interface TerminalSession {
  id: string;
  rows: number;
  cols: number;
  status: TerminalStatus;
}
