export type TerminalStatus = "Running" | "Exited";

export interface TerminalSession {
  id: string;
  rows: number;
  cols: number;
  status: TerminalStatus;
}

export interface TerminalLaunchSpec {
  launch_id: string;
  program: string;
  args: string[];
  cwd?: string;
  title?: string;
  env?: TerminalEnvVar[];
  editor: boolean;
}

export interface TerminalEnvVar {
  key: string;
  value: string;
}
