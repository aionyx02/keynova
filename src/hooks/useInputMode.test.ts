import { describe, expect, it } from "vitest";

import { parseInputMode } from "./useInputMode";

describe("parseInputMode", () => {
  it("classifies sigils without leading whitespace", () => {
    expect(parseInputMode(">ls")).toEqual({ mode: "terminal", rawInput: "ls" });
    expect(parseInputMode("/help")).toEqual({ mode: "command", rawInput: "help" });
    expect(parseInputMode("foo")).toEqual({ mode: "search", rawInput: "foo" });
  });

  // L7 regression: a leading space used to defeat terminal/command mode because
  // the sigil test ran against the untrimmed value, diverging from the
  // downstream capability parsers (which lstrip).
  it("ignores leading whitespace before the sigil", () => {
    expect(parseInputMode("  >ls")).toEqual({ mode: "terminal", rawInput: "ls" });
    expect(parseInputMode("  /help")).toEqual({ mode: "command", rawInput: "help" });
  });
});
