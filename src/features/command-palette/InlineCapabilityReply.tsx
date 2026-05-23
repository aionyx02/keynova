// REF.6.A — Inline streaming reply panel for the AI capability layer.
//
// Mounted between the result list and the palette footer in
// `CommandPalette.tsx` when an explain (or future capability) call is in
// flight. Pure presentational; `useCapability` owns the state plumbing.

import type { ReactElement } from "react";

interface Props {
  /** Accumulated streaming text from `useCapability.streamText`. */
  text: string;
  /** True while the IPC call is in flight (chip stays disabled in parent). */
  isLoading: boolean;
  /** Error string when the capability call failed or backend was offline. */
  error: string | null;
  /** Fired on the Cancel button. Parent uses it to call `cancel()` + collapse. */
  onCancel: () => void;
}

export function InlineCapabilityReply({
  text,
  isLoading,
  error,
  onCancel,
}: Props): ReactElement {
  return (
    <div className="border-t border-gray-700/50 bg-gray-950/60 px-4 py-2 text-xs text-gray-200">
      <div className="mb-1 flex items-center justify-between">
        <span className="text-[10px] uppercase tracking-wider text-gray-500">
          {error ? "Explain · error" : "Explain"}
        </span>
        <button
          type="button"
          onMouseDown={(e) => {
            e.preventDefault();
            onCancel();
          }}
          className="text-[10px] text-gray-500 hover:text-gray-300"
        >
          {isLoading ? "Cancel (Esc)" : "Close (Esc)"}
        </button>
      </div>
      {error ? (
        <div className="text-red-400 whitespace-pre-wrap break-words">{error}</div>
      ) : (
        <div className="max-h-[180px] overflow-y-auto whitespace-pre-wrap break-words leading-relaxed">
          {text || (isLoading ? "Streaming…" : "")}
        </div>
      )}
    </div>
  );
}
