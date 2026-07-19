export type InputMode = "search" | "terminal" | "command";

export interface ParsedInput {
  mode: InputMode;
  rawInput: string;
}

export function parseInputMode(value: string): ParsedInput {
  // Strip leading whitespace before testing the `>` / `/` sigils so a leading
  // space can't defeat terminal/command mode. The downstream capability parsers
  // already lstrip, so this keeps both layers consistent (L7).
  const lstripped = value.replace(/^\s+/, "");
  if (lstripped.startsWith(">")) {
    return { mode: "terminal", rawInput: lstripped.slice(1).trimStart() };
  }
  if (lstripped.startsWith("/")) {
    return { mode: "command", rawInput: lstripped.slice(1) };
  }
  return { mode: "search", rawInput: value };
}
