// REF.2.P4 — ONBOARD.1.C — Empty-state CTA shown when search mode produces
// zero raw results. Surfaces shortcuts to create a note, jump to /help,
// /setting, or replay the onboarding tour.

import { UiIcon } from "../../components/icons/UiIcon";
import { fmt } from "../../i18n/format";
import { useI18n } from "../../i18n/useI18n";

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
  const p = useI18n().palette;
  return (
    <div className="kn-panel-shell kn-empty-state overflow-hidden rounded-t-none border-t-0 px-4 py-4">
      <div className="flex items-start gap-3 text-sm text-[color:var(--kn-text-soft)]">
        <span className="flex h-10 w-10 shrink-0 items-center justify-center rounded-[8px] border border-[color:rgba(125,216,193,0.2)] bg-[color:var(--kn-accent-wash)] text-[color:var(--kn-accent)]">
          <UiIcon name="search" className="h-5 w-5" />
        </span>
        <div className="min-w-0 flex-1">
          <div className="mb-3 truncate">{fmt(p.emptyNoResults, { query })}</div>
          <div className="grid grid-cols-2 gap-2 text-xs sm:flex sm:flex-wrap">
            <button
              type="button"
              onClick={onCreateNote}
              className="kn-button kn-button-primary min-w-0"
            >
              <UiIcon name="note" className="h-3.5 w-3.5" />
              <span className="truncate">{fmt(p.createNote, { query })}</span>
            </button>
            <button type="button" onClick={onJumpHelp} className="kn-button min-w-0">
              <UiIcon name="command" className="h-3.5 w-3.5" />
              <span className="truncate">{p.tryHelp}</span>
            </button>
            <button type="button" onClick={onJumpSetting} className="kn-button min-w-0">
              <UiIcon name="settings" className="h-3.5 w-3.5" />
              <span className="truncate">{p.openSetting}</span>
            </button>
            <button type="button" onClick={onReplayOnboard} className="kn-button min-w-0">
              <UiIcon name="arrow-right" className="h-3.5 w-3.5" />
              <span className="truncate">{p.replayOnboard}</span>
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
