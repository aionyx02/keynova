// REF.2.P4 — Rendered when raw results exist but all are hidden by the
// active source-type filter chips. Surfaces the filter chips so the user
// can adjust them and a one-click clear button so the situation is not a
// dead end.

import { FilterChips } from "./FilterChips";
import { UiIcon } from "../../components/icons/UiIcon";
import { useI18n } from "../../i18n/useI18n";
import { fmt } from "../../i18n/format";
import type { SourceFilter } from "../../types/search";

interface Props {
  activeFilters: Set<SourceFilter>;
  onChangeFilters: (next: Set<SourceFilter>) => void;
  totalResults: number;
}

export function EmptyFilterState({ activeFilters, onChangeFilters, totalResults }: Props) {
  const p = useI18n().palette;
  return (
    <div className="kn-panel-shell relative overflow-hidden rounded-t-none border-t-0">
      <FilterChips active={activeFilters} onChange={onChangeFilters} />
      <div className="flex items-center gap-3 px-4 py-3 text-sm text-[color:var(--kn-text-soft)]">
        <span className="flex h-8 w-8 shrink-0 items-center justify-center rounded-[8px] border border-amber-400/20 bg-amber-400/10 text-amber-100">
          <UiIcon name="filter" className="h-4 w-4" />
        </span>
        <span className="min-w-0 flex-1">{fmt(p.filterHidesAll, { count: totalResults })}</span>
        <button
          type="button"
          onClick={() => onChangeFilters(new Set())}
          className="kn-button shrink-0 text-xs"
        >
          {p.clearFilter}
        </button>
      </div>
    </div>
  );
}
