import { useCallback, useEffect, useState } from "react";
import { useIPC } from "./useIPC";

const STEP = 10;
const FAST_STEP = 50;

const KEY_MAP: Record<string, [number, number]> = {
  ArrowUp: [0, -1],
  ArrowDown: [0, 1],
  ArrowLeft: [-1, 0],
  ArrowRight: [1, 0],
  w: [0, -1],
  s: [0, 1],
  a: [-1, 0],
  d: [1, 0],
};

export function useMouseControl() {
  const { dispatch } = useIPC();
  const [active, setActive] = useState(false);

  const toggle = useCallback(() => setActive((v) => !v), []);

  useEffect(() => {
    if (!active) return;

    async function onKeyDown(e: KeyboardEvent) {
      const dir = KEY_MAP[e.key];
      if (dir) {
        e.preventDefault();
        const step = e.shiftKey ? FAST_STEP : STEP;
        await dispatch("mouse.move_relative", { dx: dir[0] * step, dy: dir[1] * step });
      } else if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        await dispatch("mouse.click", { button: "left", count: 1 });
      }
    }

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [active, dispatch]);

  return { active, toggle };
}
