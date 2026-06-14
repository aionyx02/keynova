// Dev-only Ctrl+K -> input-ready timing instrument.
//
// Measures the webview-side latency for the palette becoming usable: from the
// frontend receiving an open signal (`window-focused` event, or first mount for
// a cold open) to the search input being focused AND painted (next animation
// frame, with `document.activeElement` confirmed to be the input).
//
// SCOPE: this is the renderer-side tail only. It does NOT include the
// OS-key-press -> webview-`Focused(true)`-event time, which would need a
// backend timestamp emitted from `app/window.rs`/`app/shortcuts.rs`. So the
// numbers here are a lower bound on the full "Ctrl+K -> ready" figure, not the
// whole thing. Recorded samples should be labelled accordingly.
//
// Everything below is a no-op in production: the public functions return early
// unless `import.meta.env.DEV`, so the buffer, console output, and `window`
// hook never exist in a release build.

export type PaletteOpenReason =
  | "cold-mount"
  | "window-focused"
  | "workspace-switched"
  | "workspace-cycled";

export interface PaletteOpenSample {
  /** Webview-side latency in milliseconds (open signal -> input painted+focused). */
  dt: number;
  /** Which open path produced this sample. */
  reason: PaletteOpenReason;
  /** Wall-clock timestamp (ms epoch) when the sample was recorded. */
  ts: number;
}

const BUFFER_CAP = 50;

/** True when the instrument should do real work. Centralised so call sites stay clean. */
export function timingEnabled(): boolean {
  return import.meta.env.DEV;
}

interface PendingOpen {
  t0: number;
  reason: PaletteOpenReason;
}

let pending: PendingOpen | null = null;
const samples: PaletteOpenSample[] = [];

type PerfListener = () => void;
const listeners = new Set<PerfListener>();

/** Subscribe to new-sample notifications (for the dev on-screen badge). Returns an unsubscribe. */
export function subscribePerf(cb: PerfListener): () => void {
  listeners.add(cb);
  return () => listeners.delete(cb);
}

/** Snapshot of the collected samples (newest last). */
export function getPerfSamples(): PaletteOpenSample[] {
  return samples.slice();
}

/** Compact stats for the dev badge: most-recent delta, median, and sample count. */
export function perfStats(): { last: number | null; median: number; n: number } {
  const ds = samples.map((s) => s.dt).sort((a, b) => a - b);
  return {
    last: samples.length > 0 ? samples[samples.length - 1].dt : null,
    median: percentile(ds, 50),
    n: ds.length,
  };
}

/** Expose the ring buffer + helpers on `window` for devtools inspection (dev only). */
function exposeOnWindow(): void {
  if (typeof window === "undefined") return;
  const w = window as unknown as {
    __keynovaPerf?: PaletteOpenSample[];
    __keynovaPerfSummary?: () => void;
  };
  w.__keynovaPerf = samples;
  w.__keynovaPerfSummary = summarizePerf;
}

/**
 * Record the start of a palette open. Last-write-wins: a newer open supersedes
 * an earlier one whose input never confirmed (e.g. a focus that got cancelled).
 */
export function markPaletteOpen(reason: PaletteOpenReason): void {
  if (!timingEnabled()) return;
  pending = { t0: performance.now(), reason };
}

/**
 * Confirm the input reached the ready state. Schedules a `requestAnimationFrame`
 * so the measurement lands after the focus has painted, then verifies the input
 * is actually the active element before recording. Safe no-op if no open is
 * pending or the input never became active.
 */
export function confirmInputReady(
  reason: PaletteOpenReason,
  inputRef: React.RefObject<HTMLInputElement | null>,
): void {
  if (!timingEnabled()) return;
  const open = pending;
  if (!open) return;

  const raf =
    typeof requestAnimationFrame === "function"
      ? requestAnimationFrame
      : (cb: FrameRequestCallback) => setTimeout(() => cb(performance.now()), 0);

  raf(() => {
    // Only count it if this input is genuinely focused at paint time.
    if (!inputRef.current || document.activeElement !== inputRef.current) return;
    // Guard against a newer open having superseded this one mid-flight.
    if (pending !== open) return;
    pending = null;

    const dt = performance.now() - open.t0;
    const sample: PaletteOpenSample = { dt, reason, ts: Date.now() };
    samples.push(sample);
    if (samples.length > BUFFER_CAP) samples.shift();
    exposeOnWindow();
    listeners.forEach((cb) => cb());

    // eslint-disable-next-line no-console
    console.info(
      `[perf] Ctrl+K input-ready: ${dt.toFixed(1)}ms (${open.reason} -> painted)`,
    );
  });
}

function percentile(sorted: number[], p: number): number {
  if (sorted.length === 0) return 0;
  const idx = Math.min(sorted.length - 1, Math.max(0, Math.ceil((p / 100) * sorted.length) - 1));
  return sorted[idx];
}

/** Print count / median / p95 / max of the collected samples. Callable from devtools. */
export function summarizePerf(): PaletteOpenSample[] {
  if (!timingEnabled()) return [];
  const ds = samples.map((s) => s.dt).sort((a, b) => a - b);
  if (ds.length === 0) {
    // eslint-disable-next-line no-console
    console.info("[perf] no Ctrl+K input-ready samples yet");
    return [];
  }
  const median = percentile(ds, 50);
  const p95 = percentile(ds, 95);
  const max = ds[ds.length - 1];
  // eslint-disable-next-line no-console
  console.info(
    `[perf] Ctrl+K input-ready over ${ds.length} samples: ` +
      `median ${median.toFixed(1)}ms, p95 ${p95.toFixed(1)}ms, max ${max.toFixed(1)}ms`,
  );
  return samples.slice();
}

/** Test-only: reset module state between cases. */
export function __resetPerfForTest(): void {
  pending = null;
  samples.length = 0;
  listeners.clear();
}
