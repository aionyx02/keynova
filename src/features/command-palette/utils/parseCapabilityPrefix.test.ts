import { describe, it, expect } from "vitest";

import { parseCapabilityPrefix } from "./parseCapabilityPrefix";

describe("parseCapabilityPrefix", () => {
  it("matches explain prefix with body", () => {
    expect(parseCapabilityPrefix("explain rust hashmap remove")).toEqual({
      id: "explain",
      args: { text: "rust hashmap remove" },
    });
  });

  it("matches summarize prefix with body", () => {
    expect(parseCapabilityPrefix("summarize the quick brown fox")).toEqual({
      id: "summarize",
      args: { text: "the quick brown fox" },
    });
  });

  it("is case-insensitive on the keyword", () => {
    expect(parseCapabilityPrefix("EXPLAIN test")).toEqual({
      id: "explain",
      args: { text: "test" },
    });
    expect(parseCapabilityPrefix("Summarize test")).toEqual({
      id: "summarize",
      args: { text: "test" },
    });
  });

  it("preserves body case", () => {
    expect(parseCapabilityPrefix("explain HashMap::remove")).toEqual({
      id: "explain",
      args: { text: "HashMap::remove" },
    });
  });

  it("trims body whitespace but keeps internal spacing", () => {
    expect(parseCapabilityPrefix("explain   rust  hashmap  ")).toEqual({
      id: "explain",
      args: { text: "rust  hashmap" },
    });
  });

  it("returns null on trailing space with no body", () => {
    expect(parseCapabilityPrefix("explain ")).toBeNull();
    expect(parseCapabilityPrefix("explain    ")).toBeNull();
  });

  it("returns null on bare keyword without trailing space", () => {
    expect(parseCapabilityPrefix("explain")).toBeNull();
    expect(parseCapabilityPrefix("summarize")).toBeNull();
  });

  it("returns null when keyword is glued to body (no separator)", () => {
    expect(parseCapabilityPrefix("explainfoo")).toBeNull();
  });

  it("returns null when keyword appears in the middle of query", () => {
    expect(parseCapabilityPrefix("git explain rust")).toBeNull();
  });

  it("returns null on empty query", () => {
    expect(parseCapabilityPrefix("")).toBeNull();
  });

  it("allows leading whitespace on query", () => {
    expect(parseCapabilityPrefix("   explain rust")).toEqual({
      id: "explain",
      args: { text: "rust" },
    });
  });

  it("returns null for prefixes not wired in this batch", () => {
    expect(parseCapabilityPrefix("next")).toBeNull();
    expect(parseCapabilityPrefix("fix some error")).toBeNull();
    expect(parseCapabilityPrefix("cmd git push origin")).toBeNull();
  });

  it("returns null for arbitrary search queries", () => {
    expect(parseCapabilityPrefix("git push origin")).toBeNull();
    expect(parseCapabilityPrefix("hashmap remove")).toBeNull();
    expect(parseCapabilityPrefix("?")).toBeNull();
  });

  it("does not match terminal/command sigils (handled upstream)", () => {
    // parseInputMode strips these before parseCapabilityPrefix sees them;
    // confirm null when the sigil leaks through unstripped.
    expect(parseCapabilityPrefix("> explain foo")).toBeNull();
    expect(parseCapabilityPrefix("/explain foo")).toBeNull();
  });
});
