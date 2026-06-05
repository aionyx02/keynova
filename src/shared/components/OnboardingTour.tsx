import { useCallback, useEffect, useState } from "react";

import { UiIcon, type UiIconName } from "../../components/icons/UiIcon";
import { fmt } from "../../i18n/format";
import { useI18n } from "../../i18n/useI18n";
// PERF.1 - Storage helpers moved to a sibling module so a lazy import of this
// component does not drag the storage keys into the eager bundle. Re-exported
// from here for callers that still reach for the old import path.
import {
  hasCompletedOnboarding,
  markOnboardingCompleted,
  resetOnboarding,
} from "./onboarding-state";

export { hasCompletedOnboarding, markOnboardingCompleted, resetOnboarding };

interface Step {
  icon: UiIconName;
}

const STEPS: Step[] = [
  { icon: "search" },
  { icon: "filter" },
  { icon: "command" },
  { icon: "settings" },
];

interface Props {
  onClose: () => void;
}

export function OnboardingTour({ onClose }: Props) {
  const t = useI18n().onboarding;
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
  const stepText = t.steps[stepIdx] ?? t.steps[0];
  const hint = fmt(t.stepHint, { current: stepIdx + 1, total: STEPS.length });
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
                  {t.badge}
                </span>
                <h2 className="mt-3 text-[22px] font-semibold tracking-tight text-[color:var(--kn-text)]">
                  {stepText.title}
                </h2>
              </div>
            </div>
            <span className="pt-1 text-[11px] font-medium text-[color:var(--kn-text-muted)]">
              {hint}
            </span>
          </div>

          <div className="h-1 overflow-hidden rounded-full bg-white/[0.06]">
            <div
              className="h-full rounded-full bg-[linear-gradient(90deg,_rgba(127,212,255,0.92)_0%,_rgba(85,188,255,0.56)_100%)] transition-[width] duration-200"
              style={{ width: progress }}
            />
          </div>
        </div>

        <div className="px-5 py-5">
          <p className="text-sm leading-7 text-[color:var(--kn-text-soft)]">{stepText.body}</p>

          <div className="mt-5 flex flex-wrap gap-2">
            <span className="kn-kbd">Esc</span>
            <span className="kn-kbd">Enter</span>
            <span className="kn-kbd">{t.leftKey}</span>
            <span className="kn-kbd">{t.rightKey}</span>
          </div>
        </div>

        <div className="flex items-center justify-between border-t border-[color:var(--kn-border)] bg-[rgba(7,11,17,0.52)] px-4 py-3 text-sm">
          <button
            type="button"
            onClick={close}
            className="font-medium text-[color:var(--kn-text-muted)] transition-colors hover:text-[color:var(--kn-text-soft)]"
          >
            {t.skip}
          </button>

          <div className="flex items-center gap-2">
            <button
              type="button"
              onClick={() => setStepIdx((index) => Math.max(0, index - 1))}
              disabled={stepIdx === 0}
              className="rounded-[12px] border border-[color:var(--kn-border)] bg-white/[0.03] px-3 py-2 font-medium text-[color:var(--kn-text-soft)] transition-colors hover:bg-white/[0.05] disabled:cursor-not-allowed disabled:opacity-35"
            >
              {t.back}
            </button>
            <button
              type="button"
              onClick={advance}
              className="inline-flex items-center gap-1.5 rounded-[12px] border border-cyan-400/20 bg-[linear-gradient(180deg,_rgba(127,212,255,0.22)_0%,_rgba(85,188,255,0.16)_100%)] px-3.5 py-2 font-semibold text-[color:var(--kn-text)] shadow-[inset_0_1px_0_rgba(255,255,255,0.16)] transition-all duration-150 hover:border-cyan-300/30 hover:bg-[linear-gradient(180deg,_rgba(127,212,255,0.28)_0%,_rgba(85,188,255,0.2)_100%)]"
            >
              {stepIdx >= STEPS.length - 1 ? t.done : t.next}
              {stepIdx < STEPS.length - 1 && <UiIcon name="arrow-right" className="h-4 w-4" />}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
