import { useCallback, useEffect, useRef } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { currentMonitor } from "@tauri-apps/api/window";
import { LogicalSize, PhysicalPosition } from "@tauri-apps/api/dpi";
import type { BuiltinCommandResult } from "./useCommands";

// Window height for the full-screen terminal mode (no input bar above).
// Covers .kn-terminal-body (460) + header (~52) + chrome.
const TERMINAL_HEIGHT_MODE = 540;
// Window height when terminal is attached below the palette input bar.
// Adds room for PaletteInputBar (~70) so the search box stays visible after
// a builtin command opens a terminal panel.
const TERMINAL_HEIGHT_ATTACHED = 620;
export const PALETTE_WIDTH_NARROW = 700;
export const PALETTE_WIDTH_WIDE = 1040;

// The palette does not fill its window. It is inset by this much on every side
// so the OS window corner falls on fully transparent pixels — Windows rounds a
// borderless window itself, and with the panel flush to the edge that system
// arc sat on top of the panel's own radius, which is what read as misaligned
// corners. The margin is also the only room the drop shadow has: sized to the
// window, it was clipped away entirely and never rendered.
//
// PALETTE_WIDTH_* stay PANEL widths. Window width is derived here, and the
// same margin has to be added to any height that is applied as a constant
// rather than measured (the measured path reads the padded container, so it
// already includes it).
export const PALETTE_MARGIN_PX = 16;

/** Window size for a given panel size. */
function windowWidthFor(panelWidth: number) {
  return panelWidth + PALETTE_MARGIN_PX * 2;
}
const PALETTE_LEFT_SHIFT_PX = 36;
const PALETTE_TOP_RATIO = 0.25;

// CONT.2 — window stability. The palette content mutates constantly (streaming
// tokens, results populating), and a native setSize per mutation makes the OS
// window chase content height and visibly jitter because the resize lags the
// webview paint. Coalesce instead: grow immediately on a meaningful change,
// ignore sub-threshold jitter, and settle briefly before shrinking so a
// transient drop (e.g. one frame between batches) does not cause grow/shrink
// flicker.
const HEIGHT_GROW_THRESHOLD_PX = 4;
const HEIGHT_SHRINK_THRESHOLD_PX = 24;
const SHRINK_SETTLE_MS = 90;

function isTerminalResult(result: BuiltinCommandResult | null) {
  return result?.ui_type.type === "Terminal";
}

/**
 * Manages window resizing: schedules RAF-based setSize calls and observes
 * DOM mutations so the window tracks content height automatically.
 *
 * `widthRef` toggles between narrow (700) and wide (1040); the
 * caller flips it when the preview pane should expand the palette horizontally.
 * Defaults to narrow when omitted.
 *
 * Returns a stable `containerRef` for the palette root element and a
 * `scheduleWindowResize` callback that callers should invoke on layout changes.
 */
export function useWindowResize(
  modeRef: React.MutableRefObject<string>,
  cmdResultRef: React.MutableRefObject<BuiltinCommandResult | null>,
  widthRef?: React.MutableRefObject<number>,
) {
  const containerRef = useRef<HTMLDivElement>(null);
  const resizeRafRef = useRef<number | null>(null);
  const shrinkTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const lastAppliedWidthRef = useRef(0);
  const lastAppliedHeightRef = useRef(0);

  // `width` is the PANEL width; the window is wider by the margin on each side.
  // `height` is already a window height — the measured path reads the padded
  // container, and the fixed terminal heights add the margin at their call site.
  const applyWindowSize = useCallback((width: number, height: number) => {
    lastAppliedWidthRef.current = width;
    lastAppliedHeightRef.current = height;
    getCurrentWindow()
      .setSize(new LogicalSize(windowWidthFor(width), height))
      .catch(() => {});
  }, []);

  const scheduleWindowPosition = useCallback(
    (widthOverride?: number) => {
      if (!window.__TAURI_INTERNALS__) return;
      const width = widthOverride ?? widthRef?.current ?? PALETTE_WIDTH_NARROW;
      void currentMonitor()
        .then((monitor) => {
          if (!monitor) return;
          const physW = Math.round(windowWidthFor(width) * monitor.scaleFactor);
          const x = Math.max(
            monitor.position.x,
            Math.round(
              monitor.position.x + (monitor.size.width - physW) / 2 - PALETTE_LEFT_SHIFT_PX,
            ),
          );
          const y = Math.round(monitor.position.y + monitor.size.height * PALETTE_TOP_RATIO);
          return getCurrentWindow()
            .setPosition(new PhysicalPosition(x, y))
            .catch(() => {});
        })
        .catch(() => {});
    },
    [widthRef],
  );

  const measureContentHeight = useCallback(() => {
    const el = containerRef.current;
    if (!el) return null;
    const rectHeight = Math.ceil(el.getBoundingClientRect().height);
    const scrollHeight = Math.ceil(el.scrollHeight);
    return Math.max(rectHeight, scrollHeight, 56);
  }, []);

  const scheduleWindowResize = useCallback(() => {
    if (!window.__TAURI_INTERNALS__) return;
    if (resizeRafRef.current !== null) {
      cancelAnimationFrame(resizeRafRef.current);
    }
    resizeRafRef.current = requestAnimationFrame(() => {
      resizeRafRef.current = null;
      const width = widthRef?.current ?? PALETTE_WIDTH_NARROW;

      // Structural modes have a deterministic height; apply directly.
      if (modeRef.current === "terminal") {
        applyWindowSize(PALETTE_WIDTH_NARROW, TERMINAL_HEIGHT_MODE + PALETTE_MARGIN_PX * 2);
        return;
      }
      if (isTerminalResult(cmdResultRef.current)) {
        applyWindowSize(PALETTE_WIDTH_NARROW, TERMINAL_HEIGHT_ATTACHED + PALETTE_MARGIN_PX * 2);
        return;
      }

      const height = measureContentHeight();
      if (height === null) return;

      // A pending shrink is superseded by any fresh measurement.
      if (shrinkTimerRef.current !== null) {
        clearTimeout(shrinkTimerRef.current);
        shrinkTimerRef.current = null;
      }

      const widthChanged = width !== lastAppliedWidthRef.current;
      const delta = height - lastAppliedHeightRef.current;

      // First sizing, a deliberate width change, or meaningful growth → apply
      // now so the surface stays responsive while typing/streaming.
      if (lastAppliedHeightRef.current === 0 || widthChanged || delta >= HEIGHT_GROW_THRESHOLD_PX) {
        applyWindowSize(width, height);
        return;
      }

      // Meaningful shrink → settle briefly so a one-frame dip between content
      // batches doesn't cause a grow/shrink flicker.
      if (delta <= -HEIGHT_SHRINK_THRESHOLD_PX) {
        shrinkTimerRef.current = setTimeout(() => {
          shrinkTimerRef.current = null;
          const settled = measureContentHeight();
          if (settled === null) return;
          applyWindowSize(widthRef?.current ?? PALETTE_WIDTH_NARROW, settled);
        }, SHRINK_SETTLE_MS);
      }
      // Otherwise the change is within the jitter threshold — skip the resize.
    });
  }, [modeRef, cmdResultRef, widthRef, applyWindowSize, measureContentHeight]);

  useEffect(() => {
    if (!window.__TAURI_INTERNALS__) return;
    const el = containerRef.current;
    if (!el) return;

    const resizeObserver = new ResizeObserver(() => scheduleWindowResize());
    resizeObserver.observe(el);

    const mutationObserver = new MutationObserver(() => scheduleWindowResize());
    mutationObserver.observe(el, { attributes: true, childList: true, subtree: true });

    scheduleWindowResize();
    scheduleWindowPosition();

    return () => {
      resizeObserver.disconnect();
      mutationObserver.disconnect();
      if (resizeRafRef.current !== null) {
        cancelAnimationFrame(resizeRafRef.current);
        resizeRafRef.current = null;
      }
      if (shrinkTimerRef.current !== null) {
        clearTimeout(shrinkTimerRef.current);
        shrinkTimerRef.current = null;
      }
    };
  }, [scheduleWindowResize, scheduleWindowPosition]);

  return { containerRef, scheduleWindowResize, scheduleWindowPosition };
}
