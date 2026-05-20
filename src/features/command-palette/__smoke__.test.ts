// Smoke test confirming Vitest + jsdom + TS pipeline is wired correctly.
// Delete this file once any real test exists in src/features/command-palette/.

import { describe, expect, it } from "vitest";

describe("vitest smoke", () => {
  it("runs", () => {
    expect(1 + 1).toBe(2);
  });

  it("has a jsdom window", () => {
    expect(typeof window).toBe("object");
    expect(typeof document).toBe("object");
  });
});
