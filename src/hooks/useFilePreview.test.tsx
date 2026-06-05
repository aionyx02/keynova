import { renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { SearchResult } from "../types/search";
import { useFilePreview } from "./useFilePreview";

const dispatchMock = vi.hoisted(() => vi.fn());

vi.mock("./useIPC", () => ({
  useIPC: () => ({ dispatch: dispatchMock }),
}));

const result: SearchResult = {
  kind: "file",
  name: "broken.docx",
  path: "C:/tmp/broken.docx",
  score: 1,
};

describe("useFilePreview", () => {
  beforeEach(() => {
    window.__TAURI_INTERNALS__ = {};
    dispatchMock.mockReset();
  });

  afterEach(() => {
    delete window.__TAURI_INTERNALS__;
  });

  it("marks failed previews so the pane can stop loading", async () => {
    dispatchMock.mockRejectedValue(new Error("bad docx"));

    const { result: hook } = renderHook(() => useFilePreview([result], 0));

    await waitFor(() => {
      expect(hook.current.failedPreviewByPath[result.path]).toBe(true);
    });
    expect(hook.current.previewByPath[result.path]).toBeUndefined();
  });
});
