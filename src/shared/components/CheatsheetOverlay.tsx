import { useEffect } from "react";

interface Binding {
  keys: string;
  desc: string;
}

interface Section {
  title: string;
  bindings: Binding[];
}

const SECTIONS: Section[] = [
  {
    title: "Global",
    bindings: [
      { keys: "Ctrl+K", desc: "Open / focus launcher" },
      { keys: "Esc", desc: "Close panel or hide window" },
      { keys: "?", desc: "Toggle this cheatsheet" },
      { keys: "/", desc: "Switch to command mode (try /help, /setting)" },
    ],
  },
  {
    title: "Search results",
    bindings: [
      { keys: "↑ / ↓", desc: "Navigate results" },
      { keys: "Enter", desc: "Open selected result" },
      { keys: "Shift+Enter", desc: "Run first secondary action" },
      { keys: "→ / Tab", desc: "Open secondary action menu" },
      { keys: "Ctrl+C", desc: "Copy path of selected result" },
    ],
  },
  {
    title: "Secondary action menu",
    bindings: [
      { keys: "↑ / ↓ / Tab", desc: "Navigate actions" },
      { keys: "Enter", desc: "Run focused action (twice for destructive)" },
      { keys: "← / Esc", desc: "Close menu" },
    ],
  },
  {
    title: "Onboarding tour",
    bindings: [
      { keys: "→ / Enter", desc: "Next step" },
      { keys: "←", desc: "Previous step" },
      { keys: "Esc", desc: "Skip / dismiss" },
    ],
  },
];

interface Props {
  onClose: () => void;
}

/** Mount only when visible; parent gates with `{open && <CheatsheetOverlay … />}`. */
export function CheatsheetOverlay({ onClose }: Props) {
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
          <span className="text-base font-semibold text-gray-100">Keyboard cheatsheet</span>
          <span className="text-[10px] uppercase tracking-wider text-gray-500">
            Esc / ? to close
          </span>
        </div>
        <div className="space-y-4 px-5 py-4">
          {SECTIONS.map((section) => (
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
