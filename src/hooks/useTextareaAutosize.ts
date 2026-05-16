import { useEffect, type RefObject } from "react";

/**
 * Auto-grow a `<textarea>` between `min` and `max` line counts based on its
 * current scroll height. Re-measures whenever `value` changes. Intended for
 * chat-style composers where the input height should adapt to multi-line
 * pastes without bumping the user past a max ceiling.
 *
 * Implementation note: we set `rows = min` and then read `scrollHeight` to
 * compute the natural row count; this avoids needing to know the exact line
 * height in pixels.
 */
export function useTextareaAutosize(
  ref: RefObject<HTMLTextAreaElement | null>,
  value: string,
  opts?: { min?: number; max?: number },
) {
  const min = opts?.min ?? 1;
  const max = opts?.max ?? 8;
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    el.rows = min;
    // Force layout reflow so scrollHeight reflects the new (shrunk) state
    // before we re-measure for the next size.
    const computed = window.getComputedStyle(el);
    const lineHeight = parseFloat(computed.lineHeight);
    if (!Number.isFinite(lineHeight) || lineHeight <= 0) {
      // Fallback: stay at min — cannot compute reliably.
      return;
    }
    const paddingTop = parseFloat(computed.paddingTop) || 0;
    const paddingBottom = parseFloat(computed.paddingBottom) || 0;
    const contentHeight = el.scrollHeight - paddingTop - paddingBottom;
    const rows = Math.min(max, Math.max(min, Math.ceil(contentHeight / lineHeight)));
    el.rows = rows;
  }, [ref, value, min, max]);
}
