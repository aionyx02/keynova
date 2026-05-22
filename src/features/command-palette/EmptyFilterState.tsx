// REF.2.P4 — Rendered when raw results exist but all are hidden by the
// active source-type filter chips. Surfaces the filter chips so the user
// can adjust them and a one-click clear button so the situation is not a
// dead end.

import { FilterChips } from "./FilterChips";
import type { SourceFilter } from "../../types/search";

interface Props {
  activeFilters: Set<SourceFilter>;
  onChangeFilters: (next: Set<SourceFilter>) => void;
  totalResults: number;
}

export function EmptyFilterState({ activeFilters, onChangeFilters, totalResults }: Props) {
  return (
    <div className="relative bg-gray-900/95 backdrop-blur-md rounded-b-xl shadow-2xl overflow-hidden">
      <FilterChips active={activeFilters} onChange={onChangeFilters} />
      <div className="px-4 py-3 text-sm text-gray-400">
        Filter hides all {totalResults} results.
        <button
          type="button"
          onClick={() => onChangeFilters(new Set())}
          className="ml-2 text-sky-300 hover:text-sky-200 underline"
        >
          Clear filter
        </button>
      </div>
    </div>
  );
}
