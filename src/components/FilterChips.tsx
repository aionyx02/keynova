import type { SourceFilter } from "../types/search";

interface ChipSpec {
  kind: SourceFilter;
  label: string;
  activeCls: string;
}

const CHIPS: ChipSpec[] = [
  { kind: "file", label: "Files", activeCls: "bg-sky-500/30 text-sky-200 ring-sky-500/40" },
  { kind: "note", label: "Notes", activeCls: "bg-teal-500/30 text-teal-200 ring-teal-500/40" },
  { kind: "app", label: "Apps", activeCls: "bg-violet-500/30 text-violet-200 ring-violet-500/40" },
  { kind: "command", label: "Commands", activeCls: "bg-emerald-500/30 text-emerald-200 ring-emerald-500/40" },
  { kind: "history", label: "History", activeCls: "bg-zinc-500/30 text-zinc-200 ring-zinc-500/40" },
  { kind: "model", label: "Models", activeCls: "bg-fuchsia-500/30 text-fuchsia-200 ring-fuchsia-500/40" },
];

const INACTIVE_CLS = "bg-gray-800/60 text-gray-400 ring-gray-700/40 hover:text-gray-200 hover:bg-gray-800";

interface Props {
  active: Set<SourceFilter>;
  onChange: (next: Set<SourceFilter>) => void;
}

export function FilterChips({ active, onChange }: Props) {
  function toggle(kind: SourceFilter) {
    const next = new Set(active);
    if (next.has(kind)) {
      next.delete(kind);
    } else {
      next.add(kind);
    }
    onChange(next);
  }

  return (
    <div className="flex flex-wrap items-center gap-1.5 border-b border-gray-700/40 bg-gray-900/60 px-3 py-1.5">
      <span className="mr-1 text-[10px] uppercase tracking-wider text-gray-500">Filter</span>
      {CHIPS.map((chip) => {
        const isActive = active.has(chip.kind);
        return (
          <button
            key={chip.kind}
            type="button"
            onClick={() => toggle(chip.kind)}
            className={`rounded-full px-2 py-0.5 text-[10px] font-semibold uppercase tracking-wide ring-1 transition-colors ${
              isActive ? chip.activeCls : INACTIVE_CLS
            }`}
            aria-pressed={isActive}
          >
            {chip.label}
          </button>
        );
      })}
      {active.size > 0 && (
        <button
          type="button"
          onClick={() => onChange(new Set())}
          className="ml-auto text-[10px] text-gray-500 hover:text-gray-300"
        >
          Clear
        </button>
      )}
    </div>
  );
}

const STORAGE_KEY = "keynova.searchFilters";

/**
 * LAUNCH.1.D — filter chips state is now in-memory only.
 *
 * Cross-session persistence (the original `loadFilters` / `saveFilters` pair)
 * created a UX trap: a single accidental click on, say, the `Notes` chip
 * would silently hide every `file` and `folder` result on every future
 * launch, with no visible cause beyond the dimly-coloured chip bar. Several
 * users hit this and reported "file and folder all gone".
 *
 * `loadFilters` now returns an empty set (always start clean). `clearLegacyFilters`
 * cleans up the leftover localStorage entry on first mount so existing users
 * recover without manual intervention.
 */
export function loadFilters(): Set<SourceFilter> {
  return new Set();
}

export function clearLegacyFilters() {
  if (typeof window === "undefined" || !window.localStorage) return;
  try {
    window.localStorage.removeItem(STORAGE_KEY);
  } catch {
    /* ignore */
  }
}

