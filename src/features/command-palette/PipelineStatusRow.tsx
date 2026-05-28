import type { PipelineReport } from "./hooks/usePipeline";

interface Props {
  running: boolean;
  result: PipelineReport | null;
}

function statusTone(status: string): string {
  return status === "completed" ? "bg-emerald-400 text-emerald-100" : "bg-rose-400 text-rose-100";
}

export function PipelineStatusRow({ running, result }: Props) {
  if (running) {
    return (
      <div className="kn-panel-shell flex items-center gap-3 rounded-t-none border-t-0 px-4 py-3">
        <span className="h-2.5 w-2.5 animate-pulse rounded-full bg-[color:var(--kn-accent)]" />
        <span className="text-sm font-medium text-[color:var(--kn-text-soft)]">
          Pipeline running...
        </span>
      </div>
    );
  }

  if (!result) return null;

  const isCompleted = result.log.status === "completed";

  return (
    <div className="kn-panel-shell overflow-hidden rounded-t-none border-t-0">
      <ul className="kn-scroll max-h-[220px] overflow-y-auto px-2 py-2">
        {result.actions.map((stage) => (
          <li
            key={stage.index}
            className="flex items-start gap-3 rounded-[12px] px-3 py-2 text-sm text-[color:var(--kn-text-soft)]"
          >
            <span className={`mt-1.5 h-2 w-2 shrink-0 rounded-full ${statusTone(stage.status)}`} />
            <span className="shrink-0 font-mono text-xs text-[color:var(--kn-text-muted)]">
              {stage.route}
            </span>
            {stage.error && <span className="min-w-0 truncate text-rose-200">{stage.error}</span>}
          </li>
        ))}
        {result.log.status === "failed" && result.log.error && result.actions.length === 0 && (
          <li className="px-3 py-2 text-sm text-rose-200">{result.log.error}</li>
        )}
      </ul>
      <div className="kn-panel-footer">
        <span className={isCompleted ? "text-emerald-300" : "text-rose-300"}>
          {result.log.status}
        </span>
        <span>{result.log.action_count} stages</span>
      </div>
    </div>
  );
}
