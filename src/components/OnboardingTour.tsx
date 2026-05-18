import { useEffect, useState } from "react";

const STORAGE_KEY = "keynova.onboarding.completed";

interface Step {
  title: string;
  body: string;
  hint?: string;
}

const STEPS: Step[] = [
  {
    title: "Welcome to Keynova",
    body: "A keyboard-first launcher. Press Ctrl+K (or Cmd+K on macOS) anywhere to bring this palette up.",
    hint: "1 / 4",
  },
  {
    title: "Search files, apps, notes",
    body: "Just start typing. Results stream in as you go. Use ↑/↓ to navigate, Enter to open, → for the secondary action menu.",
    hint: "2 / 4",
  },
  {
    title: "Run commands with /",
    body: "Type `/` to switch to command mode. Try `/help`, `/setting`, `/cal`, or `/uuid`. Tab completes suggestions.",
    hint: "3 / 4",
  },
  {
    title: "Customise & explore",
    body: "Open /setting to rebind hotkeys, enable AI, or tune indexing. Press `?` any time to see keybindings. Run `/onboard` to replay this tour.",
    hint: "4 / 4",
  },
];

/** Returns true when the user has already completed (or skipped) the tour. */
export function hasCompletedOnboarding(): boolean {
  if (typeof window === "undefined" || !window.localStorage) return true;
  try {
    return window.localStorage.getItem(STORAGE_KEY) === "true";
  } catch {
    return true;
  }
}

export function markOnboardingCompleted() {
  if (typeof window === "undefined" || !window.localStorage) return;
  try {
    window.localStorage.setItem(STORAGE_KEY, "true");
  } catch {
    /* ignore */
  }
}

/** Clears the completed flag so the tour replays on next mount. */
export function resetOnboarding() {
  if (typeof window === "undefined" || !window.localStorage) return;
  try {
    window.localStorage.removeItem(STORAGE_KEY);
  } catch {
    /* ignore */
  }
}

interface Props {
  onClose: () => void;
}

/**
 * Mount only when the tour should be visible. Conditional mount in the parent
 * (`{open && <OnboardingTour ... />}`) guarantees `stepIdx` resets to 0 on
 * every reopen without needing a setState-in-effect reset hook.
 */
export function OnboardingTour({ onClose }: Props) {
  const [stepIdx, setStepIdx] = useState(0);

  function advance() {
    if (stepIdx >= STEPS.length - 1) {
      close();
    } else {
      setStepIdx((i) => i + 1);
    }
  }

  function close() {
    markOnboardingCompleted();
    onClose();
  }

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopPropagation();
        close();
      } else if (e.key === "Enter" || e.key === "ArrowRight") {
        e.preventDefault();
        e.stopPropagation();
        advance();
      } else if (e.key === "ArrowLeft") {
        e.preventDefault();
        e.stopPropagation();
        setStepIdx((i) => Math.max(0, i - 1));
      }
    };
    window.addEventListener("keydown", handler, true);
    return () => window.removeEventListener("keydown", handler, true);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [stepIdx]);

  const step = STEPS[stepIdx];
  return (
    <div
      className="fixed inset-0 z-40 flex items-center justify-center bg-black/60 backdrop-blur-sm"
      role="dialog"
      aria-modal="true"
    >
      <div className="w-[440px] rounded-xl border border-gray-700/60 bg-gray-900/95 shadow-2xl">
        <div className="flex items-baseline justify-between px-5 pt-4 pb-2">
          <span className="text-base font-semibold text-gray-100">{step.title}</span>
          {step.hint && (
            <span className="text-[10px] uppercase tracking-wider text-gray-500">{step.hint}</span>
          )}
        </div>
        <p className="px-5 pb-4 text-sm leading-relaxed text-gray-300">{step.body}</p>
        <div className="flex items-center justify-between border-t border-gray-700/50 px-4 py-2 text-xs">
          <button
            type="button"
            onClick={close}
            className="text-gray-500 hover:text-gray-300"
          >
            Skip
          </button>
          <div className="flex items-center gap-2">
            <button
              type="button"
              onClick={() => setStepIdx((i) => Math.max(0, i - 1))}
              disabled={stepIdx === 0}
              className="rounded px-2 py-1 text-gray-400 hover:bg-gray-800/60 hover:text-gray-200 disabled:opacity-30 disabled:hover:bg-transparent"
            >
              Back
            </button>
            <button
              type="button"
              onClick={advance}
              className="rounded bg-sky-600 px-3 py-1 font-medium text-white hover:bg-sky-500"
            >
              {stepIdx >= STEPS.length - 1 ? "Got it" : "Next →"}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
