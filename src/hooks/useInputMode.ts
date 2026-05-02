export type InputMode = "search" | "terminal" | "command";

export interface ParsedInput {
  mode: InputMode;
  rawInput: string;
}

export function parseInputMode(value: string): ParsedInput {
  if (value.startsWith(">")) {
    return { mode: "terminal", rawInput: value.slice(1).trimStart() };
  }
  if (value.startsWith("/")) {
    return { mode: "command", rawInput: value };
  }
  return { mode: "search", rawInput: value };
}
