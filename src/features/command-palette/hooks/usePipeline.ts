// Automation pipeline state owned by the command palette.
//
// Encapsulates the `automation.execute_pipeline` IPC call and the
// running/result UI flags. Pipeline runs when the user submits a query
// containing `|`; the result row renders the per-stage log returned by
// the backend.

import { useCallback, useState } from "react";

import type { DispatchFn } from "../../../context/IPCContext";

export interface PipelineStageResult {
  index: number;
  route: string;
  status: string;
  output?: unknown;
  error?: string;
}

export interface PipelineReport {
  log: {
    workflow_name: string;
    status: string;
    action_count: number;
    error?: string;
  };
  actions: PipelineStageResult[];
}

export interface UsePipelineDeps {
  /** IPC dispatcher; matches `useIPC().dispatch`. */
  dispatch: DispatchFn;
}

export interface UsePipeline {
  pipelineResult: PipelineReport | null;
  pipelineRunning: boolean;
  /** Run a pipeline against the supplied text query (assumed to contain `|`). */
  runPipeline: (text: string) => Promise<void>;
  /** Reset both state flags (used on ESC / query change). */
  clear: () => void;
}

export function usePipeline({ dispatch }: UsePipelineDeps): UsePipeline {
  const [pipelineResult, setPipelineResult] = useState<PipelineReport | null>(null);
  const [pipelineRunning, setPipelineRunning] = useState(false);

  const runPipeline = useCallback(
    async (text: string) => {
      setPipelineRunning(true);
      setPipelineResult(null);
      try {
        const report = await dispatch<PipelineReport>("automation.execute_pipeline", { text });
        setPipelineResult(report);
      } catch (err) {
        setPipelineResult({
          log: { workflow_name: "pipeline", status: "failed", action_count: 0, error: String(err) },
          actions: [],
        });
      } finally {
        setPipelineRunning(false);
      }
    },
    // dispatch is intentionally omitted — useIPC returns a fresh wrapper each render
    // and the runPipeline closure must still read the latest dispatch via the param.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [],
  );

  const clear = useCallback(() => {
    setPipelineResult(null);
    setPipelineRunning(false);
  }, []);

  return { pipelineResult, pipelineRunning, runPipeline, clear };
}
