import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import type { BuiltinCommandResult } from "./useCommands";

export type AgentRisk = "low" | "medium" | "high";

export type ContextVisibility =
  | "public_context"
  | "user_private"
  | "private_architecture"
  | "secret";

export type AgentMemoryScope = "session" | "workspace" | "long_term";

export interface GroundingSource {
  source_id: string;
  source_type: string;
  title: string;
  snippet: string;
  uri?: string | null;
  score: number;
  visibility: ContextVisibility;
  redacted_reason?: string | null;
}

export interface AgentFilteredSource {
  source_id: string;
  source_type: string;
  title: string;
  visibility: ContextVisibility;
  reason: string;
}

export interface AgentPromptAudit {
  budget_chars: number;
  prompt_chars: number;
  truncated: boolean;
  included_sources: GroundingSource[];
  filtered_sources: AgentFilteredSource[];
  redacted_secret_count: number;
}

export interface AgentToolCall {
  id: string;
  tool_name: string;
  risk: AgentRisk;
  status: string;
  duration_ms?: number | null;
  error?: string | null;
}

export interface AgentStep {
  id: string;
  title: string;
  status: string;
  tool_calls: AgentToolCall[];
}

export interface AgentMemoryRef {
  id: string;
  scope: AgentMemoryScope;
  visibility: ContextVisibility;
  summary: string;
}

export interface AgentPlannedAction {
  id: string;
  kind:
    | "open_panel"
    | "create_note_draft"
    | "update_setting_draft"
    | "run_builtin_command"
    | "terminal_command"
    | "file_write"
    | "system_control"
    | "model_lifecycle";
  risk: AgentRisk;
  label: string;
  summary: string;
  payload: unknown;
}

export interface AgentRun {
  id: string;
  prompt: string;
  status: "planning" | "waiting_approval" | "running" | "completed" | "cancelled" | "failed";
  plan: string[];
  output?: string;
  error?: string;
  steps: AgentStep[];
  approvals: AgentApproval[];
  memory_refs: AgentMemoryRef[];
  sources: GroundingSource[];
  prompt_audit?: AgentPromptAudit | null;
  command_result?: BuiltinCommandResult | null;
}

export interface AgentApproval {
  id: string;
  summary: string;
  risk: AgentRisk;
  status: string;
  planned_action?: AgentPlannedAction | null;
}

interface AgentEventPayload {
  run_id: string;
  status: AgentRun["status"];
  run: AgentRun;
}

async function ipcDispatch<T>(route: string, payload?: Record<string, unknown>): Promise<T> {
  return invoke<T>("cmd_dispatch", { route, payload: payload ?? null });
}

function upsertRunChronologically(prev: AgentRun[], run: AgentRun): AgentRun[] {
  const index = prev.findIndex((item) => item.id === run.id);
  if (index === -1) return [...prev, run];
  return prev.map((item) => (item.id === run.id ? run : item));
}

export function useAgent() {
  const [runs, setRuns] = useState<AgentRun[]>([]);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!window.__TAURI_INTERNALS__) return;
    const updateFromEvent = (event: { payload: AgentEventPayload }) => {
      setRuns((prev) => upsertRunChronologically(prev, event.payload.run));
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
      setRuns((prev) => upsertRunChronologically(prev, run));
    } finally {
      setLoading(false);
    }
  }, []);

  const cancel = useCallback(async (runId: string) => {
    const run = await ipcDispatch<AgentRun>("agent.cancel", { run_id: runId });
    setRuns((prev) => upsertRunChronologically(prev, run));
  }, []);

  const approve = useCallback(async (runId: string, approvalId: string) => {
    const run = await ipcDispatch<AgentRun>("agent.approve", {
      run_id: runId,
      approval_id: approvalId,
    });
    setRuns((prev) => upsertRunChronologically(prev, run));
  }, []);

  const reject = useCallback(async (runId: string, approvalId: string) => {
    const run = await ipcDispatch<AgentRun>("agent.reject", {
      run_id: runId,
      approval_id: approvalId,
    });
    setRuns((prev) => upsertRunChronologically(prev, run));
  }, []);

  return { runs, loading, start, cancel, approve, reject };
}
