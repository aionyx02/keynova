// REF.6.A — chip + Explain wiring for SearchResultsList.

import { render, screen, fireEvent } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { SearchResult } from "../../types/search";
import type { UnifiedResult } from "../../types/unified-result";
import { SearchResultsList } from "./SearchResultsList";

function makeLegacy(): SearchResult {
  return {
    kind: "file",
    name: "foo.rs",
    title: "foo.rs",
    subtitle: "/tmp/foo.rs",
    path: "/tmp/foo.rs",
    score: 100,
    icon_key: null,
    secondary_action_count: 0,
    unified_id: "u-1",
  };
}

function makeUnified(): UnifiedResult {
  return {
    id: "u-1",
    source: { type: "file", kind: "file", path: "/tmp/foo.rs" },
    title: "foo.rs",
    subtitle: "/tmp/foo.rs",
    actions: [],
    rank: { score: 100 },
  };
}

const baseProps = {
  visibleResults: [makeLegacy()],
  unifiedVisible: [makeUnified()],
  safeSelected: 0,
  iconsByKey: {},
  onSelectIndex: vi.fn(),
  onLaunch: vi.fn(),
  showRankBreakdown: false,
  onHoverStart: vi.fn(),
  onHoverEnd: vi.fn(),
  activeFilters: new Set<never>() as Set<never>,
  onChangeFilters: vi.fn(),
  secondaryMenuOpen: false,
  selectedResult: makeLegacy(),
  menuItems: [],
  menuFocusedIndex: 0,
  pendingConfirm: null,
  inlineInput: null,
  onSecondaryAction: vi.fn(),
  onMenuFocus: vi.fn(),
  onInlineInputChange: vi.fn(),
  onInlineInputKeyDown: vi.fn(),
  showPreview: false,
  previewForSelected: undefined,
  previewLoading: false,
  expandedMetadata: false,
  selectedMetadata: null,
  footerHint: "" as unknown as React.ReactNode,
};

describe("SearchResultsList — Explain chip (REF.6.A)", () => {
  it("renders an Explain chip on every visible row", () => {
    const onExplain = vi.fn();
    render(
      <SearchResultsList
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        {...(baseProps as any)}
        onExplain={onExplain}
        explainLoading={false}
      />,
    );
    expect(screen.getByText("Explain")).not.toBeNull();
  });

  it("clicking the chip calls onExplain with the row's title", () => {
    const onExplain = vi.fn();
    render(
      <SearchResultsList
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        {...(baseProps as any)}
        onExplain={onExplain}
        explainLoading={false}
      />,
    );
    fireEvent.mouseDown(screen.getByText("Explain"));
    expect(onExplain).toHaveBeenCalledWith("foo.rs");
  });

  it("chip is disabled while explainLoading is true on the focused row", () => {
    const onExplain = vi.fn();
    render(
      <SearchResultsList
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        {...(baseProps as any)}
        onExplain={onExplain}
        explainLoading={true}
      />,
    );
    // The chip label collapses to "…" while disabled.
    const chip = screen.getByRole("button", { name: /Explain foo\.rs/i });
    expect((chip as HTMLButtonElement).disabled).toBe(true);
    fireEvent.mouseDown(chip);
    expect(onExplain).not.toHaveBeenCalled();
  });

  it("chip click does not propagate to row onLaunch", () => {
    const onLaunch = vi.fn();
    const onExplain = vi.fn();
    render(
      <SearchResultsList
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        {...(baseProps as any)}
        onLaunch={onLaunch}
        onExplain={onExplain}
        explainLoading={false}
      />,
    );
    fireEvent.mouseDown(screen.getByText("Explain"));
    expect(onLaunch).not.toHaveBeenCalled();
  });
});
