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
const PALETTE_LEFT_SHIFT_PX = 36;
const PALETTE_TOP_RATIO = 0.25;

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
  const scheduleWindowPosition = useCallback(
    (widthOverride?: number) => {
      if (!window.__TAURI_INTERNALS__) return;
      const width = widthOverride ?? widthRef?.current ?? PALETTE_WIDTH_NARROW;
      void currentMonitor()
        .then((monitor) => {
          if (!monitor) return;
          const physW = Math.round(width * monitor.scaleFactor);
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

  const scheduleWindowResize = useCallback(() => {
    if (!window.__TAURI_INTERNALS__) return;
    if (resizeRafRef.current !== null) {
      cancelAnimationFrame(resizeRafRef.current);
    }
    resizeRafRef.current = requestAnimationFrame(() => {
      resizeRafRef.current = null;
      const width = widthRef?.current ?? PALETTE_WIDTH_NARROW;
      if (modeRef.current === "terminal") {
        getCurrentWindow()
          .setSize(new LogicalSize(PALETTE_WIDTH_NARROW, TERMINAL_HEIGHT_MODE))
          .catch(() => {});
        return;
      }
      if (isTerminalResult(cmdResultRef.current)) {
        getCurrentWindow()
          .setSize(new LogicalSize(PALETTE_WIDTH_NARROW, TERMINAL_HEIGHT_ATTACHED))
          .catch(() => {});
        return;
      }
      const el = containerRef.current;
      if (!el) return;
      const rectHeight = Math.ceil(el.getBoundingClientRect().height);
      const scrollHeight = Math.ceil(el.scrollHeight);
      const height = Math.max(rectHeight, scrollHeight, 56);
      getCurrentWindow()
        .setSize(new LogicalSize(width, height))
        .catch(() => {});
    });
  }, [modeRef, cmdResultRef, widthRef]);

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
    };
  }, [scheduleWindowResize, scheduleWindowPosition]);

  return { containerRef, scheduleWindowResize, scheduleWindowPosition };
}
