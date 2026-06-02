// Dispatcher between the prefix mode kind and the matching
// capability surface. Renders within the palette body slot when
// `usePaletteMode` returns `{ kind: "capability", ... }`.
//
// The stream is lifted into `CommandPalette` so the global Esc handler can
// reach `stream.cancel` without an imperative-handle dance. This component
// stays pure presentational and switches on `id` to pick the right card.

import type { ReactElement } from "react";

import { CapabilityAnswerCard } from "../ai-capability/CapabilityAnswerCard";
import { CapabilityCommandCard } from "../ai-capability/CapabilityCommandCard";
import { CapabilityListCard } from "../ai-capability/CapabilityListCard";
import type { CapabilityRunStatus } from "../ai-capability/hooks/useCapabilityRunState";
import type { UseCapabilityStream } from "../ai-capability/hooks/useCapabilityStream";
import type { GenCommandOutput, SuggestedNextAction } from "../ai-capability/types";
import type { DispatchFn } from "../../context/IPCContext";

export type CapabilitySurfaceMode =
  | { id: "explain" | "summarize" | "fix"; args: { text: string }; source: "prefix" | "smart" }
  | { id: "cmd"; args: { text: string }; source: "prefix" | "smart" }
  | { id: "next"; args: Record<string, never>; source: "prefix" | "smart" };

interface Props {
  /** Active capability surface (explicit prefix or smart auto-surface). */
  mode: CapabilitySurfaceMode;
  /** Lifted stream state from CommandPalette. */
  answerStream: UseCapabilityStream;
  commandCard: {
    status: CapabilityRunStatus;
    data: GenCommandOutput | null;
    error: string | null;
    startedAtMs: number | null;
    completedAtMs: number | null;
    riskRequiresConfirmation: boolean;
    onSubmit: () => void;
    onCancel: () => void;
    onEditBefore: (command: string) => void;
    onRun: (command: string) => void;
  };
  listCard: {
    status: CapabilityRunStatus;
    items: SuggestedNextAction[];
    error: string | null;
    startedAtMs: number | null;
    completedAtMs: number | null;
    selectedIndex: number;
    onSelectIndex: (index: number) => void;
    onRunSelected: (index: number) => void;
    onCancel: () => void;
  };
  dispatch: DispatchFn;
  /** Called when the user closes the card via the `[×]` button; clears the
   * prefix from the palette input so the card unmounts. */
  onClose: () => void;
}

export function CapabilityResultArea({
  mode,
  answerStream,
  commandCard,
  listCard,
  dispatch,
  onClose,
}: Props): ReactElement {
  switch (mode.id) {
    case "explain":
    case "summarize":
    case "fix":
      return (
        <CapabilityAnswerCard
          capabilityLabel={mode.id}
          status={answerStream.status}
          text={answerStream.text}
          error={answerStream.error}
          startedAtMs={answerStream.startedAtMs}
          firstChunkAtMs={answerStream.firstChunkAtMs}
          completedAtMs={answerStream.completedAtMs}
          args={mode.args}
          dispatch={dispatch}
          onCancel={answerStream.cancel}
          onClose={onClose}
        />
      );
    case "cmd":
      return (
        <CapabilityCommandCard
          label={mode.source === "smart" ? "command" : "cmd"}
          status={commandCard.status}
          data={commandCard.data}
          error={commandCard.error}
          startedAtMs={commandCard.startedAtMs}
          completedAtMs={commandCard.completedAtMs}
          intent={mode.args.text}
          riskRequiresConfirmation={commandCard.riskRequiresConfirmation}
          onCancel={commandCard.onCancel}
          onClose={onClose}
          onSubmit={commandCard.onSubmit}
          onRun={commandCard.onRun}
          onEditBefore={commandCard.onEditBefore}
        />
      );
    case "next":
      return (
        <CapabilityListCard
          label={mode.source === "smart" ? "next step" : "next"}
          status={listCard.status}
          items={listCard.items}
          error={listCard.error}
          startedAtMs={listCard.startedAtMs}
          completedAtMs={listCard.completedAtMs}
          selectedIndex={listCard.selectedIndex}
          onSelectIndex={listCard.onSelectIndex}
          onRunSelected={listCard.onRunSelected}
          onCancel={listCard.onCancel}
          onClose={onClose}
        />
      );
  }
}
