import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { IPC } from "../ipc/routes";
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

export interface ReactStep {
  step: number;
  tool_name: string | null;
  status: string;
  observation_preview?: string | null;
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

export interface WorkspaceContext {
  id: string;
  name: string;
  project_root?: string | null;
  recent_file_count: number;
  note_count: number;
}

export interface SelectedFileContext {
  path: string;
  preview: string;
}

export interface ContextTokenBudget {
  budget_chars: number;
  used_chars: number;
  remaining_chars: number;
  truncated: boolean;
}

export interface ContextBundle {
  user_intent: string;
  workspace: WorkspaceContext;
  recent_actions: string[];
  selected_files: SelectedFileContext[];
  search_results: GroundingSource[];
  token_budget: ContextTokenBudget;
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
  context_bundle?: ContextBundle | null;
  command_result?: BuiltinCommandResult | null;
}

export interface AgentApproval {
  id: string;
  summary: string;
  risk: AgentRisk;
  status: string;
  planned_action?: AgentPlannedAction | null;
  tool_name?: string | null;
  deadline_unix_ms?: number | null;
  remember_for_run?: boolean;
}

interface AgentEventPayload {
  run_id: string;
  status: AgentRun["status"];
  run: AgentRun;
}

interface AgentStepEventPayload {
  run_id: string;
  step: number;
  tool_name: string | null;
  status: string;
  observation_preview?: string | null;
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
  const [reactSteps, setReactSteps] = useState<Record<string, ReactStep[]>>({});
  const [archivedCount, setArchivedCount] = useState(0);

  useEffect(() => {
    if (!window.__TAURI_INTERNALS__) return;
    const updateFromEvent = (event: { payload: AgentEventPayload }) => {
      setRuns((prev) => upsertRunChronologically(prev, event.payload.run));
      setLoading(false);
    };
    const onArchived = (event: { payload: AgentEventPayload }) => {
      // Backend evicted the oldest in-memory run to agent_archive; drop it
      // from the panel state and bump the archive counter for UI hint.
      setRuns((prev) => prev.filter((r) => r.id !== event.payload.run.id));
      setArchivedCount((n) => n + 1);
    };
    const updateStep = (event: { payload: AgentStepEventPayload }) => {
      const { run_id, step, tool_name, status, observation_preview } = event.payload;
      setReactSteps((prev) => {
        const existing = prev[run_id] ?? [];
        const idx = existing.findIndex((s) => s.step === step && s.tool_name === tool_name);
        const updated: ReactStep = { step, tool_name, status, observation_preview };
        const next =
          idx === -1
            ? [...existing, updated].sort((a, b) => a.step - b.step)
            : existing.map((s, i) => (i === idx ? updated : s));
        return { ...prev, [run_id]: next };
      });
    };
    const listeners = Promise.all([
      listen<AgentEventPayload>("agent-run-started", updateFromEvent),
      listen<AgentEventPayload>("agent-approval-required", updateFromEvent),
      // Backend emits agent.approval.timeout (→ "agent-approval-timeout") when
      // a pending approval crosses its deadline; the run object includes the
      // updated approval status so we route it through the standard updater.
      listen<AgentEventPayload>("agent-approval-timeout", updateFromEvent),
      listen<AgentEventPayload>("agent-run-completed", updateFromEvent),
      listen<AgentEventPayload>("agent-run-failed", updateFromEvent),
      listen<AgentEventPayload>("agent-run-archived", onArchived),
      listen<AgentStepEventPayload>("agent-step", updateStep),
    ]);
    return () => {
      listeners.then((fns) => fns.forEach((fn) => fn()));
    };
  }, []);

  const start = useCallback(async (prompt: string) => {
    if (!prompt.trim()) return;
    setLoading(true);
    try {
      const run = await ipcDispatch<AgentRun>(IPC.AGENT_START, { prompt });
      setRuns((prev) => upsertRunChronologically(prev, run));
    } finally {
      setLoading(false);
    }
  }, []);

  const cancel = useCallback(async (runId: string) => {
    const run = await ipcDispatch<AgentRun>(IPC.AGENT_CANCEL, { run_id: runId });
    setRuns((prev) => upsertRunChronologically(prev, run));
  }, []);

  const approve = useCallback(
    async (runId: string, approvalId: string, remember?: boolean) => {
      const run = await ipcDispatch<AgentRun>(IPC.AGENT_APPROVE, {
        run_id: runId,
        approval_id: approvalId,
        remember: remember ?? false,
      });
      setRuns((prev) => upsertRunChronologically(prev, run));
    },
    [],
  );

  const reject = useCallback(async (runId: string, approvalId: string) => {
    const run = await ipcDispatch<AgentRun>(IPC.AGENT_REJECT, {
      run_id: runId,
      approval_id: approvalId,
    });
    setRuns((prev) => upsertRunChronologically(prev, run));
  }, []);

  const clearRuns = useCallback(async () => {
    await ipcDispatch<{ ok: boolean }>(IPC.AGENT_CLEAR_RUNS);
    setRuns([]);
    setReactSteps({});
    setLoading(false);
    setArchivedCount(0);
  }, []);

  const resetArchivedCount = useCallback(() => setArchivedCount(0), []);

  return {
    runs,
    loading,
    start,
    cancel,
    approve,
    reject,
    clearRuns,
    reactSteps,
    archivedCount,
    resetArchivedCount,
  };
}
