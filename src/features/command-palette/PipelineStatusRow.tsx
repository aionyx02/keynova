// REF.2.P4 — Pipeline running indicator + completed pipeline report row.
//
// Two render paths in one component because the running indicator and the
// result list never coexist (running -> result -> Esc-cleared), and they
// share the same outer card styling.

import type { PipelineReport } from "./hooks/usePipeline";

interface Props {
  running: boolean;
  result: PipelineReport | null;
}

export function PipelineStatusRow({ running, result }: Props) {
  if (running) {
    return (
      <div className="bg-gray-900/95 backdrop-blur-md rounded-b-xl shadow-2xl px-4 py-3">
        <span className="text-sm text-blue-400 animate-pulse">Pipeline running…</span>
      </div>
    );
  }

  if (!result) return null;

  return (
    <div className="bg-gray-900/95 backdrop-blur-md rounded-b-xl shadow-2xl overflow-hidden">
      <ul className="py-1">
        {result.actions.map((stage) => (
          <li key={stage.index} className="flex items-start gap-2 px-4 py-1.5 text-sm">
            <span
              className={`shrink-0 font-mono text-xs mt-0.5 ${
                stage.status === "completed" ? "text-emerald-400" : "text-red-400"
              }`}
            >
              {stage.status === "completed" ? "✓" : "✗"}
            </span>
            <span className="font-mono text-gray-400 shrink-0">{stage.route}</span>
            {stage.error && <span className="text-red-400 truncate">{stage.error}</span>}
          </li>
        ))}
        {result.log.status === "failed" &&
          result.log.error &&
          result.actions.length === 0 && (
            <li className="px-4 py-1.5 text-sm text-red-400">{result.log.error}</li>
          )}
      </ul>
      <div className="border-t border-gray-700/50 px-4 py-1.5 text-[11px] text-gray-600 flex justify-between">
        <span
          className={result.log.status === "completed" ? "text-emerald-600" : "text-red-600"}
        >
          {result.log.status}
        </span>
        <span>{result.log.action_count} stages</span>
        <span>Esc 清除</span>
      </div>
    </div>
  );
}
