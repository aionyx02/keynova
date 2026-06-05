import { UiIcon, type UiIconName } from "../../components/icons/UiIcon";
import { useI18n } from "../../i18n/useI18n";
import type { SourceFilter } from "../../types/search";

interface ChipSpec {
  kind: SourceFilter;
  icon: UiIconName;
}

const CHIPS: ChipSpec[] = [
  { kind: "file", icon: "file" },
  { kind: "note", icon: "note" },
  { kind: "app", icon: "app" },
  { kind: "command", icon: "command" },
  { kind: "history", icon: "history" },
  { kind: "model", icon: "model" },
];

const STORAGE_KEY = "keynova.searchFilters";

interface Props {
  active: Set<SourceFilter>;
  onChange: (next: Set<SourceFilter>) => void;
}

export function FilterChips({ active, onChange }: Props) {
  const p = useI18n().palette;

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
    <div className="flex flex-wrap items-center gap-2 border-b border-[color:var(--kn-border)] bg-white/[0.02] px-3 py-2">
      <span className="mr-1 inline-flex items-center gap-1.5 text-[10px] uppercase tracking-[0.2em] text-[color:var(--kn-text-faint)]">
        <UiIcon name="filter" className="h-3.5 w-3.5" />
        {p.scope}
      </span>
      {CHIPS.map((chip) => {
        const isActive = active.has(chip.kind);
        return (
          <button
            key={chip.kind}
            type="button"
            onClick={() => toggle(chip.kind)}
            aria-pressed={isActive}
            className={`kn-chip ${
              isActive ? "kn-chip-active shadow-[inset_0_1px_0_rgba(255,255,255,0.06)]" : ""
            }`}
          >
            <UiIcon name={chip.icon} className="h-3.5 w-3.5" />
            {p.filterLabels[chip.kind]}
          </button>
        );
      })}
      {active.size > 0 && (
        <button
          type="button"
          onClick={() => onChange(new Set())}
          className="ml-auto inline-flex items-center gap-1.5 text-[11px] font-medium text-[color:var(--kn-text-muted)] transition-colors hover:text-[color:var(--kn-text-soft)]"
        >
          <UiIcon name="x" className="h-3.5 w-3.5" />
          {p.clear}
        </button>
      )}
    </div>
  );
}

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
