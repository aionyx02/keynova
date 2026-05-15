import { useCallback, useEffect, useRef } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { LogicalSize } from "@tauri-apps/api/dpi";
import type { BuiltinCommandResult } from "./useCommands";

const TERMINAL_HEIGHT = 360;

function isTerminalResult(result: BuiltinCommandResult | null) {
  return result?.ui_type.type === "Terminal";
}

/**
 * Manages window resizing: schedules RAF-based setSize calls and observes
 * DOM mutations so the window tracks content height automatically.
 *
 * Returns a stable `containerRef` for the palette root element and a
 * `scheduleWindowResize` callback that callers should invoke on layout changes.
 */
export function useWindowResize(
  modeRef: React.MutableRefObject<string>,
  cmdResultRef: React.MutableRefObject<BuiltinCommandResult | null>,
) {
  const containerRef = useRef<HTMLDivElement>(null);
  const resizeRafRef = useRef<number | null>(null);

  const scheduleWindowResize = useCallback(() => {
    if (!window.__TAURI_INTERNALS__) return;
    if (resizeRafRef.current !== null) {
      cancelAnimationFrame(resizeRafRef.current);
    }
    resizeRafRef.current = requestAnimationFrame(() => {
      resizeRafRef.current = null;
      if (modeRef.current === "terminal") {
        getCurrentWindow().setSize(new LogicalSize(640, TERMINAL_HEIGHT)).catch(() => {});
        return;
      }
      if (isTerminalResult(cmdResultRef.current)) {
        getCurrentWindow().setSize(new LogicalSize(640, TERMINAL_HEIGHT)).catch(() => {});
        return;
      }
      const el = containerRef.current;
      if (!el) return;
      const rectHeight = Math.ceil(el.getBoundingClientRect().height);
      const scrollHeight = Math.ceil(el.scrollHeight);
      const height = Math.max(rectHeight, scrollHeight, 56);
      getCurrentWindow().setSize(new LogicalSize(640, height)).catch(() => {});
    });
  }, [modeRef, cmdResultRef]);

  useEffect(() => {
    if (!window.__TAURI_INTERNALS__) return;
    const el = containerRef.current;
    if (!el) return;

    const resizeObserver = new ResizeObserver(() => scheduleWindowResize());
    resizeObserver.observe(el);

    const mutationObserver = new MutationObserver(() => scheduleWindowResize());
    mutationObserver.observe(el, { attributes: true, childList: true, subtree: true });

    scheduleWindowResize();

    return () => {
      resizeObserver.disconnect();
      mutationObserver.disconnect();
      if (resizeRafRef.current !== null) {
        cancelAnimationFrame(resizeRafRef.current);
        resizeRafRef.current = null;
      }
    };
  }, [scheduleWindowResize]);

  return { containerRef, scheduleWindowResize };
}