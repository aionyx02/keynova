import { useCallback, useEffect, useState } from "react";

import { UiIcon, type UiIconName } from "../../components/icons/UiIcon";

const STORAGE_KEY = "keynova.onboarding.completed";

interface Step {
  title: string;
  body: string;
  hint?: string;
  icon: UiIconName;
}

const STEPS: Step[] = [
  {
    title: "Welcome to Keynova",
    body: "A keyboard-first launcher for fast local actions. Press Ctrl+K, start typing, and stay in flow.",
    hint: "1 / 4",
    icon: "search",
  },
  {
    title: "Search with almost no friction",
    body: "Type naturally to search files, apps, notes, and history. Use Up or Down to move and Enter to open.",
    hint: "2 / 4",
    icon: "filter",
  },
  {
    title: "Use slash commands when intent is clear",
    body: "Type / to switch into command mode. Try /help, /setting, /cal, or /uuid when you want direct actions.",
    hint: "3 / 4",
    icon: "command",
  },
  {
    title: "Tune the workspace around you",
    body: "Open /setting to adjust hotkeys, indexing, and AI features. Press ? any time to review keybindings.",
    hint: "4 / 4",
    icon: "settings",
  },
];

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

export function OnboardingTour({ onClose }: Props) {
  const [stepIdx, setStepIdx] = useState(0);

  const close = useCallback(() => {
    markOnboardingCompleted();
    onClose();
  }, [onClose]);

  const advance = useCallback(() => {
    if (stepIdx >= STEPS.length - 1) {
      close();
    } else {
      setStepIdx((index) => index + 1);
    }
  }, [close, stepIdx]);

  useEffect(() => {
    const handler = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        event.stopPropagation();
        close();
      } else if (event.key === "Enter" || event.key === "ArrowRight") {
        event.preventDefault();
        event.stopPropagation();
        advance();
      } else if (event.key === "ArrowLeft") {
        event.preventDefault();
        event.stopPropagation();
        setStepIdx((index) => Math.max(0, index - 1));
      }
    };

    window.addEventListener("keydown", handler, true);
    return () => window.removeEventListener("keydown", handler, true);
  }, [advance, close]);

  const step = STEPS[stepIdx];
  const progress = `${((stepIdx + 1) / STEPS.length) * 100}%`;

  return (
    <div
      className="fixed inset-0 z-40 flex items-center justify-center bg-black/60 backdrop-blur-sm"
      role="dialog"
      aria-modal="true"
    >
      <div className="kn-panel-shell w-[460px] overflow-hidden rounded-[22px]">
        <div className="border-b border-[color:var(--kn-border)] bg-[rgba(7,11,17,0.52)] px-5 py-4">
          <div className="mb-3 flex items-start justify-between gap-4">
            <div className="flex items-start gap-3">
              <span className="flex h-11 w-11 shrink-0 items-center justify-center rounded-[14px] border border-cyan-400/20 bg-cyan-400/10 text-cyan-200 shadow-[inset_0_1px_0_rgba(255,255,255,0.12)]">
                <UiIcon name={step.icon} className="h-5 w-5" />
              </span>
              <div>
                <span className="inline-flex rounded-full border border-cyan-400/20 bg-cyan-400/10 px-2.5 py-1 text-[10px] font-semibold uppercase tracking-[0.18em] text-cyan-200">
                  Getting Started
                </span>
                <h2 className="mt-3 text-[22px] font-semibold tracking-tight text-[color:var(--kn-text)]">
                  {step.title}
                </h2>
              </div>
            </div>
            {step.hint && (
              <span className="pt-1 text-[11px] font-medium text-[color:var(--kn-text-muted)]">
                {step.hint}
              </span>
            )}
          </div>

          <div className="h-1 overflow-hidden rounded-full bg-white/[0.06]">
            <div
              className="h-full rounded-full bg-[linear-gradient(90deg,_rgba(127,212,255,0.92)_0%,_rgba(85,188,255,0.56)_100%)] transition-[width] duration-200"
              style={{ width: progress }}
            />
          </div>
        </div>

        <div className="px-5 py-5">
          <p className="text-sm leading-7 text-[color:var(--kn-text-soft)]">{step.body}</p>

          <div className="mt-5 flex flex-wrap gap-2">
            <span className="kn-kbd">Esc</span>
            <span className="kn-kbd">Enter</span>
            <span className="kn-kbd">Left</span>
            <span className="kn-kbd">Right</span>
          </div>
        </div>

        <div className="flex items-center justify-between border-t border-[color:var(--kn-border)] bg-[rgba(7,11,17,0.52)] px-4 py-3 text-sm">
          <button
            type="button"
            onClick={close}
            className="font-medium text-[color:var(--kn-text-muted)] transition-colors hover:text-[color:var(--kn-text-soft)]"
          >
            Skip
          </button>

          <div className="flex items-center gap-2">
            <button
              type="button"
              onClick={() => setStepIdx((index) => Math.max(0, index - 1))}
              disabled={stepIdx === 0}
              className="rounded-[12px] border border-[color:var(--kn-border)] bg-white/[0.03] px-3 py-2 font-medium text-[color:var(--kn-text-soft)] transition-colors hover:bg-white/[0.05] disabled:cursor-not-allowed disabled:opacity-35"
            >
              Back
            </button>
            <button
              type="button"
              onClick={advance}
              className="inline-flex items-center gap-1.5 rounded-[12px] border border-cyan-400/20 bg-[linear-gradient(180deg,_rgba(127,212,255,0.22)_0%,_rgba(85,188,255,0.16)_100%)] px-3.5 py-2 font-semibold text-[color:var(--kn-text)] shadow-[inset_0_1px_0_rgba(255,255,255,0.16)] transition-all duration-150 hover:border-cyan-300/30 hover:bg-[linear-gradient(180deg,_rgba(127,212,255,0.28)_0%,_rgba(85,188,255,0.2)_100%)]"
            >
              {stepIdx >= STEPS.length - 1 ? "Done" : "Next"}
              {stepIdx < STEPS.length - 1 && <UiIcon name="arrow-right" className="h-4 w-4" />}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
