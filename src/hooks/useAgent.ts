import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";

export interface AgentRun {
  id: string;
  prompt: string;
  status: "planning" | "waiting_approval" | "running" | "completed" | "cancelled" | "failed";
  plan: string[];
  output?: string;
  error?: string;
}

interface AgentEventPayload {
  run_id: string;
  status: AgentRun["status"];
  run: AgentRun;
}

async function ipcDispatch<T>(route: string, payload?: Record<string, unknown>): Promise<T> {
  return invoke<T>("cmd_dispatch", { route, payload: payload ?? null });
}

export function useAgent() {
  const [runs, setRuns] = useState<AgentRun[]>([]);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!window.__TAURI_INTERNALS__) return;
    const unlisten = listen<AgentEventPayload>("agent-run-completed", (event) => {
      setRuns((prev) => [event.payload.run, ...prev.filter((run) => run.id !== event.payload.run_id)]);
      setLoading(false);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const start = useCallback(async (prompt: string) => {
    if (!prompt.trim()) return;
    setLoading(true);
    try {
      const run = await ipcDispatch<AgentRun>("agent.start", { prompt });
      setRuns((prev) => [run, ...prev.filter((item) => item.id !== run.id)]);
    } finally {
      setLoading(false);
    }
  }, []);

  const cancel = useCallback(async (runId: string) => {
    const run = await ipcDispatch<AgentRun>("agent.cancel", { run_id: runId });
    setRuns((prev) => [run, ...prev.filter((item) => item.id !== run.id)]);
  }, []);

  return { runs, loading, start, cancel };
}

