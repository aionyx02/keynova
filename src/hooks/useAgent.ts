import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import type { BuiltinCommandResult } from "./useCommands";

export interface AgentRun {
  id: string;
  prompt: string;
  status: "planning" | "waiting_approval" | "running" | "completed" | "cancelled" | "failed";
  plan: string[];
  output?: string;
  error?: string;
  approvals: AgentApproval[];
  command_result?: BuiltinCommandResult | null;
}

export interface AgentApproval {
  id: string;
  summary: string;
  risk: "low" | "medium" | "high";
  status: string;
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
    const updateFromEvent = (event: { payload: AgentEventPayload }) => {
      setRuns((prev) => [event.payload.run, ...prev.filter((run) => run.id !== event.payload.run_id)]);
      setLoading(false);
    };
    const listeners = Promise.all([
      listen<AgentEventPayload>("agent-run-started", updateFromEvent),
      listen<AgentEventPayload>("agent-approval-required", updateFromEvent),
      listen<AgentEventPayload>("agent-run-completed", updateFromEvent),
      listen<AgentEventPayload>("agent-run-failed", updateFromEvent),
    ]);
    return () => {
      listeners.then((fns) => fns.forEach((fn) => fn()));
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

  const approve = useCallback(async (runId: string, approvalId: string) => {
    const run = await ipcDispatch<AgentRun>("agent.approve", {
      run_id: runId,
      approval_id: approvalId,
    });
    setRuns((prev) => [run, ...prev.filter((item) => item.id !== run.id)]);
  }, []);

  const reject = useCallback(async (runId: string, approvalId: string) => {
    const run = await ipcDispatch<AgentRun>("agent.reject", {
      run_id: runId,
      approval_id: approvalId,
    });
    setRuns((prev) => [run, ...prev.filter((item) => item.id !== run.id)]);
  }, []);

  return { runs, loading, start, cancel, approve, reject };
}
