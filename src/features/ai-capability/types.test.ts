import { describe, expect, it } from "vitest";

import { parseGenCommandOutput, parseSuggestNextOutput } from "./types";

describe("ai capability structured parsers", () => {
  it("parses gen_command structured payloads", () => {
    expect(
      parseGenCommandOutput({
        kind: "structured",
        value: {
          command: "git status",
          confidence: 0.9,
          rationale: "Inspect changes first.",
        },
      }),
    ).toEqual({
      command: "git status",
      confidence: 0.9,
      rationale: "Inspect changes first.",
    });
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
