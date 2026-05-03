import type { ITheme } from "@xterm/xterm";

const DARK_THEME: ITheme = {
  background: "#111827",
  foreground: "#e5e7eb",
  cursor: "#60a5fa",
  cursorAccent: "#111827",
  selectionBackground: "#3b82f650",
  black: "#1f2937",
  red: "#f87171",
  green: "#4ade80",
  yellow: "#fbbf24",
  blue: "#60a5fa",
  magenta: "#c084fc",
  cyan: "#22d3ee",
  white: "#e5e7eb",
  brightBlack: "#374151",
  brightRed: "#fca5a5",
  brightGreen: "#86efac",
  brightYellow: "#fde68a",
  brightBlue: "#93c5fd",
  brightMagenta: "#d8b4fe",
  brightCyan: "#67e8f9",
  brightWhite: "#f9fafb",
};

export interface TerminalOptions {
  theme: ITheme;
  fontFamily: string;
  fontSize: number;
  cursorStyle: "block" | "underline" | "bar";
  cursorBlink: boolean;
  scrollback: number;
}

export function useTerminalTheme(): TerminalOptions {
  return {
    theme: DARK_THEME,
    fontFamily: '"Cascadia Code", Consolas, "Courier New", monospace',
    fontSize: 13,
    cursorStyle: "block",
    cursorBlink: true,
    scrollback: 1000,
  };
}
