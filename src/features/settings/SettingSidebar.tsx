// Section rail for the settings window.
//
// A vertical list on the left rather than a horizontal tab strip along the top.
// Sections are a stable set of names people scan rather than a small set of
// modes they flip between, which is why every settings window on every platform
// looks like this — and a vertical list keeps growing without the horizontal
// scrolling (and the fade masks that hid it) the tab strip needed.
//
// The rail is a real focus stop, not decoration. ArrowLeft from a row enters
// it, ArrowUp / ArrowDown move through the sections — switching as they go, so
// the content always matches the highlight rather than trailing a separate
// cursor — and ArrowRight or Enter drops back into the rows.

interface SettingSidebarProps {
  sections: string[];
  activeSection: string;
  /** `settings.sectionLabels` from the active locale. */
  labels: Record<string, string>;
  /** True while the filter is non-empty: rows then come from every section, so
   *  no single entry is current and the rail shows nothing as selected. */
  filtering: boolean;
  onSelect: (section: string) => void;
  onMove: (dir: 1 | -1) => void;
  onEnterRows: () => void;
  registerRef: (el: HTMLButtonElement | null, index: number) => void;
}

/** Falls back to a capitalised raw key so a section the locale has not been
 *  taught yet still reads as a name rather than disappearing. */
export function sectionDisplayLabel(section: string, labels: Record<string, string>): string {
  const mapped = labels[section];
  if (mapped) return mapped;
  if (!section) return section;
  return section.charAt(0).toUpperCase() + section.slice(1);
}

export function SettingSidebar({
  sections,
  activeSection,
  labels,
  filtering,
  onSelect,
  onMove,
  onEnterRows,
  registerRef,
}: SettingSidebarProps) {
  return (
    <nav className="kn-scroll w-44 shrink-0 overflow-y-auto border-r border-[color:var(--kn-border)] bg-[color:var(--kn-panel-bg-soft)] py-2">
      {sections.map((section, index) => {
        const active = !filtering && section === activeSection;
        return (
          <button
            key={section}
            ref={(el) => registerRef(el, index)}
            type="button"
            aria-current={active ? "true" : undefined}
            onClick={() => onSelect(section)}
            onKeyDown={(e) => {
              if (e.key === "ArrowDown") {
                e.preventDefault();
                onMove(1);
              } else if (e.key === "ArrowUp") {
                e.preventDefault();
                onMove(-1);
              } else if (e.key === "ArrowRight" || e.key === "Enter") {
                e.preventDefault();
                onEnterRows();
              }
            }}
            className={`flex w-full items-center border-l-2 px-3 py-1.5 text-left text-[12px] font-medium transition-colors ${
              active
                ? "border-[color:var(--kn-accent)] bg-[color:var(--kn-selected)] text-[color:var(--kn-text)]"
                : "border-transparent text-[color:var(--kn-text-faint)] hover:bg-[color:var(--kn-panel-bg-elevated)] hover:text-[color:var(--kn-text-soft)]"
            }`}
          >
            {sectionDisplayLabel(section, labels)}
          </button>
        );
      })}
    </nav>
  );
}
