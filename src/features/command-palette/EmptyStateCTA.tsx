// REF.2.P4 — ONBOARD.1.C — Empty-state CTA shown when search mode produces
// zero raw results. Surfaces shortcuts to create a note, jump to /help,
// /setting, or replay the onboarding tour.

import { UiIcon } from "../../components/icons/UiIcon";

interface Props {
  query: string;
  onCreateNote: () => void;
  onJumpHelp: () => void;
  onJumpSetting: () => void;
  onReplayOnboard: () => void;
}

export function EmptyStateCTA({
  query,
  onCreateNote,
  onJumpHelp,
  onJumpSetting,
  onReplayOnboard,
}: Props) {
  return (
    <div className="kn-panel-shell overflow-hidden rounded-t-none border-t-0 px-4 py-4">
      <div className="text-sm text-[color:var(--kn-text-soft)]">
        <div className="mb-3">
          No results for <span className="font-mono text-[color:var(--kn-text)]">{query}</span>.
        </div>
        <div className="flex flex-wrap gap-2 text-xs">
          <button type="button" onClick={onCreateNote} className="kn-button kn-button-primary">
            <UiIcon name="note" className="h-3.5 w-3.5" />
            Create note &quot;{query}&quot;
          </button>
          <button type="button" onClick={onJumpHelp} className="kn-button">
            <UiIcon name="command" className="h-3.5 w-3.5" />
            Try /help
          </button>
          <button type="button" onClick={onJumpSetting} className="kn-button">
            <UiIcon name="settings" className="h-3.5 w-3.5" />
            Open /setting
          </button>
          <button type="button" onClick={onReplayOnboard} className="kn-button">
            <UiIcon name="arrow-right" className="h-3.5 w-3.5" />
            Replay /onboard
          </button>
        </div>
      </div>
    </div>
  );
}
