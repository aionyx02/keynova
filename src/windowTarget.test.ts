import { describe, expect, it } from "vitest";

import { resolveWindowTarget } from "./windowTarget";

describe("resolveWindowTarget", () => {
  it("routes the settings window", () => {
    expect(resolveWindowTarget("?window=settings")).toBe("settings");
  });

  it("routes the launcher when there is no query at all", () => {
    expect(resolveWindowTarget("")).toBe("launcher");
  });

  it("falls back to the launcher for an unknown window", () => {
    expect(resolveWindowTarget("?window=nope")).toBe("launcher");
  });

  it("ignores unrelated query parameters", () => {
    expect(resolveWindowTarget("?debug=1&window=settings&x=2")).toBe("settings");
    expect(resolveWindowTarget("?debug=1")).toBe("launcher");
  });
});
