import { useEffect } from "react";

import { useI18n } from "../../i18n/useI18n";

interface Binding {
  keys: string;
  desc: string;
}

interface Section {
  title: string;
  bindings: Binding[];
}

interface Props {
  onClose: () => void;
}

/** Mount only when visible; parent gates with `{open && <CheatsheetOverlay … />}`. */
export function CheatsheetOverlay({ onClose }: Props) {
  const t = useI18n().cheatsheet;
  const s = t.sections;
  const sections: Section[] = [
    {
      title: s.globalTitle,
      bindings: [
        { keys: "Ctrl+K", desc: s.openFocusLauncher },
        { keys: "Esc", desc: s.closePanelOrHideWindow },
        { keys: "?", desc: s.toggleCheatsheet },
        { keys: "/", desc: s.switchCommandMode },
      ],
    },
    {
      title: s.searchResultsTitle,
      bindings: [
        { keys: "↑ / ↓", desc: s.navigateResults },
        { keys: "Enter", desc: s.openSelectedResult },
        { keys: "Shift+Enter", desc: s.runFirstSecondaryAction },
        { keys: "→ / Tab", desc: s.openSecondaryActionMenu },
        { keys: "Ctrl+C", desc: s.copySelectedPath },
      ],
    },
    {
      title: s.secondaryActionMenuTitle,
      bindings: [
        { keys: "↑ / ↓ / Tab", desc: s.navigateActions },
        { keys: "Enter", desc: s.runFocusedAction },
        { keys: "← / Esc", desc: s.closeMenu },
      ],
    },
    {
      title: s.onboardingTitle,
      bindings: [
        { keys: "→ / Enter", desc: s.nextStep },
        { keys: "←", desc: s.previousStep },
        { keys: "Esc", desc: s.skipDismiss },
      ],
    },
  ];

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape" || e.key === "?") {
        e.preventDefault();
        e.stopPropagation();
        onClose();
      }
    };
    window.addEventListener("keydown", handler, true);
    return () => window.removeEventListener("keydown", handler, true);
  }, [onClose]);

  return (
    <div
      className="fixed inset-0 z-40 flex items-center justify-center bg-black/60 backdrop-blur-sm"
      role="dialog"
      aria-modal="true"
      onClick={onClose}
    >
      <div
        className="w-[520px] max-h-[80vh] overflow-y-auto rounded-xl border border-gray-700/60 bg-gray-900/95 shadow-2xl"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-baseline justify-between border-b border-gray-700/50 px-5 py-3">
          <span className="text-base font-semibold text-gray-100">{t.title}</span>
          <span className="text-[10px] uppercase tracking-wider text-gray-500">{t.closeHint}</span>
        </div>
        <div className="space-y-4 px-5 py-4">
          {sections.map((section) => (
            <div key={section.title}>
              <div className="mb-1 text-[10px] uppercase tracking-wider text-gray-500">
                {section.title}
              </div>
              <div className="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1 text-xs">
                {section.bindings.map((b) => (
                  <BindingRow key={b.keys} binding={b} />
                ))}
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

function BindingRow({ binding }: { binding: Binding }) {
  return (
    <>
      <span className="font-mono text-sky-300">{binding.keys}</span>
      <span className="text-gray-300">{binding.desc}</span>
    </>
  );
}
