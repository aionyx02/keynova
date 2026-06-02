import { UiIcon, type UiIconName } from "../../components/icons/UiIcon";
import type { SourceFilter } from "../../types/search";

interface ChipSpec {
  kind: SourceFilter;
  label: string;
  icon: UiIconName;
}

const CHIPS: ChipSpec[] = [
  { kind: "file", label: "Files", icon: "file" },
  { kind: "note", label: "Notes", icon: "note" },
  { kind: "app", label: "Apps", icon: "app" },
  { kind: "command", label: "Commands", icon: "command" },
  { kind: "history", label: "History", icon: "history" },
  { kind: "model", label: "Models", icon: "model" },
];

const STORAGE_KEY = "keynova.searchFilters";

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
    <div className="flex flex-wrap items-center gap-2 border-b border-[color:var(--kn-border)] bg-white/[0.02] px-3 py-2">
      <span className="mr-1 inline-flex items-center gap-1.5 text-[10px] uppercase tracking-[0.2em] text-[color:var(--kn-text-faint)]">
        <UiIcon name="filter" className="h-3.5 w-3.5" />
        Scope
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
            {chip.label}
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
          Clear
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
