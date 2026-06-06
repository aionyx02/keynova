import { describe, expect, it } from "vitest";

import {
  parseCapabilitySources,
  parseFixErrorOutput,
  parseGenCommandOutput,
  parseRecallOutput,
  parseRememberOutput,
  parseSuggestNextOutput,
} from "./types";

describe("ai capability structured parsers", () => {
  it("parses gen_command structured payloads", () => {
    expect(
      parseGenCommandOutput({
        kind: "structured",
        value: {
          command: "git status",
          confidence: 0.9,
          rationale: "Inspect changes first.",
          assumptions: {
            cwd: "C:/work/keynova",
            shell: "powershell",
            os: "windows",
          },
        },
      }),
    ).toEqual({
      command: "git status",
      confidence: 0.9,
      rationale: "Inspect changes first.",
      assumptions: {
        cwd: "C:/work/keynova",
        shell: "powershell",
        os: "windows",
      },
    });
  });

  it("parses fix_error structured replies", () => {
    expect(
      parseFixErrorOutput({
        kind: "structured",
        value: {
          explanation: "The compiler found a type mismatch.",
          suggested_command: {
            command: "cargo check",
            confidence: 0.6,
            rationale: "Re-run the compiler.",
          },
        },
      }),
    ).toEqual({
      explanation: "The compiler found a type mismatch.",
      suggested_command: {
        command: "cargo check",
        confidence: 0.6,
        rationale: "Re-run the compiler.",
      },
    });
  });

  it("drops malformed source rows", () => {
    expect(
      parseCapabilitySources([
        { source_id: "workspace:1", source_type: "workspace", title: "Keynova" },
        { source_id: "bad" },
      ]),
    ).toEqual([
      {
        source_id: "workspace:1",
        source_type: "workspace",
        title: "Keynova",
        uri: null,
      },
    ]);
  });

  it("rejects malformed gen_command payloads", () => {
    expect(
      parseGenCommandOutput({
        kind: "structured",
        value: { command: "git status", confidence: "high" },
      }),
    ).toBeNull();
  });

  it("parses suggest_next structured arrays", () => {
    expect(
      parseSuggestNextOutput({
        kind: "structured",
        value: [
          {
            title: "/help",
            subtitle: "cmd.run · recent · same context",
            route: "cmd.run",
            confidence: 0.88,
            rationale: "recent command usage",
            last_executed_at: 123,
            workspace_id: 7,
            replay: {
              route: "cmd.run",
              payload: { name: "help", args: "" },
            },
          },
        ],
      }),
    ).toEqual([
      {
        title: "/help",
        subtitle: "cmd.run · recent · same context",
        route: "cmd.run",
        confidence: 0.88,
        rationale: "recent command usage",
        last_executed_at: 123,
        workspace_id: 7,
        replay: {
          route: "cmd.run",
          payload: { name: "help", args: "" },
        },
        action_ref: undefined,
      },
    ]);
  });

  it("parses remember structured payloads", () => {
    expect(
      parseRememberOutput({
        kind: "structured",
        value: { id: "abc", title: "Coffee order", content: "- oat flat white", saved: true },
      }),
    ).toEqual({
      id: "abc",
      title: "Coffee order",
      content: "- oat flat white",
      saved: true,
    });
  });

  it("rejects malformed remember payloads", () => {
    expect(
      parseRememberOutput({ kind: "structured", value: { id: "abc", title: "x", saved: "yes" } }),
    ).toBeNull();
  });

  it("parses recall structured arrays and drops malformed rows", () => {
    const parsed = parseRecallOutput({
      kind: "structured",
      value: [
        {
          id: "1",
          title: "Coffee",
          snippet: "oat flat white",
          content: "oat flat white",
          score: 3,
        },
        { id: "2", title: "bad" },
      ],
    });
    expect(parsed).toEqual([
      { id: "1", title: "Coffee", snippet: "oat flat white", content: "oat flat white", score: 3 },
    ]);
  });

  it("drops malformed suggest_next rows", () => {
    expect(
      parseSuggestNextOutput({
        kind: "structured",
        value: [
          { title: "/help" },
          {
            title: "/setting",
            subtitle: "cmd.run",
            route: "cmd.run",
            confidence: 0.7,
            rationale: "recent command usage",
            last_executed_at: 22,
          },
        ],
      }),
    ).toHaveLength(1);
  });
});
