// Row-level rendering tests. The placeholder note (REF.6.A/.B) is replaced by
// UX.AUDIT.5 coverage: the mojibake (`嚙`) guard must keep rows actionable by
// falling back to the raw path instead of masking the detail line.

import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";

import { SearchResultsList } from "./SearchResultsList";
import type { SearchResult, SourceFilter } from "../../types/search";
import type { UnifiedResult } from "../../types/unified-result";

function makeResult(over: Partial<SearchResult>): SearchResult {
  return {
    kind: "file",
    name: "report",
    path: "C:/clean/path/report.txt",
    score: 1,
    ...over,
  };
}

function makeUnified(id: string): UnifiedResult {
  return { id, source: { type: "other", name: id }, title: id, subtitle: "", actions: [] };
}

function renderList(results: SearchResult[]) {
  return render(
    <SearchResultsList
      visibleResults={results}
      unifiedVisible={results.map((_, i) => makeUnified(`u${i}`))}
      safeSelected={0}
      iconsByKey={{}}
      onSelectIndex={vi.fn()}
      onLaunch={vi.fn()}
      showRankBreakdown={false}
      onHoverStart={vi.fn()}
      onHoverEnd={vi.fn()}
      activeFilters={new Set<SourceFilter>()}
      onChangeFilters={vi.fn()}
      secondaryMenuOpen={false}
      selectedResult={null}
      menuItems={[]}
      menuFocusedIndex={0}
      pendingConfirm={null}
      inlineInput={null}
      onSecondaryAction={vi.fn()}
      onMenuFocus={vi.fn()}
      onInlineInputChange={vi.fn()}
      onInlineInputKeyDown={vi.fn()}
      showPreview={false}
      previewForSelected={undefined}
      previewLoading={false}
      expandedMetadata={false}
      selectedMetadata={null}
      footerHint={null}
    />,
  );
}

describe("SearchResultsList encoding fallback (UX.AUDIT.5)", () => {
  it("shows the raw path when the detail line trips the mojibake guard", () => {
    renderList([
      makeResult({ subtitle: "嚙嚙嚙報告", path: "C:/projects/app/report.txt" }),
    ]);
    // Raw path stays visible so the row remains actionable…
    expect(screen.getByText("C:/projects/app/report.txt")).toBeTruthy();
    // …and the old "Path unavailable" mask is gone.
    expect(screen.queryByText(/unavailable/i)).toBeNull();
  });

  it("still masks a garbled title but keeps the path beneath it", () => {
    renderList([
      makeResult({ title: "嚙嚙標題", subtitle: undefined, path: "C:/data/notes.md" }),
    ]);
    // Garbled title is masked to the localized placeholder…
    expect(screen.getByText("Unavailable text")).toBeTruthy();
    // …while the path (detail source) still identifies the row.
    expect(screen.getByText("C:/data/notes.md")).toBeTruthy();
  });

  it("renders clean titles and details untouched", () => {
    renderList([makeResult({ title: "Quarterly Report", subtitle: "C:/q4/report.txt" })]);
    expect(screen.getByText("Quarterly Report")).toBeTruthy();
    expect(screen.getByText("C:/q4/report.txt")).toBeTruthy();
  });
});
