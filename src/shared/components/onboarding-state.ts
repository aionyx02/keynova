// PERF.1 — Onboarding completion state extracted from `OnboardingTour.tsx` so
// the heavy tour component itself can be lazy-loaded without pulling its
// `hasCompletedOnboarding` / `resetOnboarding` helpers into the eager bundle.
//
// Behavior is identical to the prior implementation: localStorage backed,
// missing/unavailable storage is treated as "already completed" so first-run
// flows don't trip in test environments or restrictive sandboxes.

const STORAGE_KEY = "keynova.onboarding.completed";

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
