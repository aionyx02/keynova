import { renderHook } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { ReactNode } from "react";

import { IPCProvider } from "./IPCContext";
import { FeatureFlagsProvider, useFeatureFlags } from "./FeatureFlagsContext";

// jsdom path: `window.__TAURI_INTERNALS__` is absent, so the provider skips the
// `setting.list_all` fetch and the seeded defaults stand. This is exactly the
// pre-fetch state in the real app, so it must already gate correctly: AI is
// opt-in (hidden), the rest are opt-out (shown).

function wrapper({ children }: { children: ReactNode }) {
  return (
    <IPCProvider>
      <FeatureFlagsProvider>{children}</FeatureFlagsProvider>
    </IPCProvider>
  );
}

describe("FeatureFlagsContext", () => {
  it("seeds AI off and other features on before any backend read", () => {
    const { result } = renderHook(() => useFeatureFlags(), { wrapper });
    expect(result.current.isEnabled("ai")).toBe(false);
    expect(result.current.isEnabled("notes")).toBe(true);
    expect(result.current.isEnabled("history")).toBe(true);
    expect(result.current.isEnabled("translation")).toBe(true);
    expect(result.current.isEnabled("calculator")).toBe(true);
    expect(result.current.isEnabled("system")).toBe(true);
  });
});
