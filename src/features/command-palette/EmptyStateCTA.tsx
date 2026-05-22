// REF.2.P4 — ONBOARD.1.C — Empty-state CTA shown when search mode produces
// zero raw results. Surfaces shortcuts to create a note, jump to /help,
// /setting, or replay the onboarding tour.

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
    <div className="bg-gray-900/95 backdrop-blur-md rounded-b-xl shadow-2xl overflow-hidden">
      <div className="px-4 py-3 text-sm text-gray-400">
        <div className="mb-2">
          No results for <span className="font-mono text-gray-300">{query}</span>.
        </div>
        <div className="flex flex-wrap gap-2 text-xs">
          <button
            type="button"
            onClick={onCreateNote}
            className="rounded bg-sky-600/30 px-2 py-1 text-sky-200 ring-1 ring-sky-600/40 hover:bg-sky-600/50"
          >
            Create note &quot;{query}&quot;
          </button>
          <button
            type="button"
            onClick={onJumpHelp}
            className="rounded bg-gray-800/60 px-2 py-1 text-gray-300 ring-1 ring-gray-700/40 hover:bg-gray-800"
          >
            Try /help
          </button>
          <button
            type="button"
            onClick={onJumpSetting}
            className="rounded bg-gray-800/60 px-2 py-1 text-gray-300 ring-1 ring-gray-700/40 hover:bg-gray-800"
          >
            Open /setting
          </button>
          <button
            type="button"
            onClick={onReplayOnboard}
            className="rounded bg-gray-800/60 px-2 py-1 text-gray-300 ring-1 ring-gray-700/40 hover:bg-gray-800"
          >
            Replay /onboard
          </button>
        </div>
      </div>
    </div>
  );
}
