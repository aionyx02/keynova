import type { ITheme } from "@xterm/xterm";

const DARK_THEME: ITheme = {
  background: "#0b1017",
  foreground: "#d7e2ef",
  cursor: "#7fd4ff",
  cursorAccent: "#0b1017",
  selectionBackground: "#7fd4ff33",
  black: "#121a24",
  red: "#fb7185",
  green: "#5ee6a8",
  yellow: "#f8c96b",
  blue: "#7bb7ff",
  magenta: "#d8a6ff",
  cyan: "#79dcff",
  white: "#d7e2ef",
  brightBlack: "#46566a",
  brightRed: "#fda4af",
  brightGreen: "#86efac",
  brightYellow: "#fde68a",
  brightBlue: "#a8cbff",
  brightMagenta: "#e9d5ff",
  brightCyan: "#baeaff",
  brightWhite: "#f8fbff",
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
