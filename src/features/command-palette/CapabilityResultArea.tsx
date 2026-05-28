// REF.6.B — Dispatcher between the prefix mode kind and the matching
// capability surface. Renders within the palette body slot when
// `usePaletteMode` returns `{ kind: "capability", ... }`.
//
// The stream is lifted into `CommandPalette` so the global Esc handler can
// reach `stream.cancel` without an imperative-handle dance. This component
// stays pure presentational and switches on `id` to pick the right card.

import type { ReactElement } from "react";

import { CapabilityAnswerCard } from "../ai-capability/CapabilityAnswerCard";
import type { UseCapabilityStream } from "../ai-capability/hooks/useCapabilityStream";
import type { DispatchFn } from "../../context/IPCContext";
import type { CapabilityPrefixId } from "./utils/parseCapabilityPrefix";

interface Props {
  /** Active capability prefix and its args, from `usePaletteMode`. */
  id: CapabilityPrefixId;
  args: { text: string };
  /** Lifted stream state from CommandPalette. */
  stream: UseCapabilityStream;
  dispatch: DispatchFn;
  /** Called when the user closes the card via the `[×]` button; clears the
   * prefix from the palette input so the card unmounts. */
  onClose: () => void;
}

export function CapabilityResultArea({
  id,
  args,
  stream,
  dispatch,
  onClose,
}: Props): ReactElement {
  // Today both wired prefixes (`explain`, `summarize`) map to the same card.
  // The exhaustive switch will surface as a type error when REF.6.D / .E / .F
  // add new `CapabilityPrefixId` variants.
  switch (id) {
    case "explain":
    case "summarize":
      return (
        <CapabilityAnswerCard
          capabilityLabel={id}
          status={stream.status}
          text={stream.text}
          error={stream.error}
          startedAtMs={stream.startedAtMs}
          firstChunkAtMs={stream.firstChunkAtMs}
          completedAtMs={stream.completedAtMs}
          args={args}
          dispatch={dispatch}
          onCancel={stream.cancel}
          onClose={onClose}
        />
      );
  }
}
