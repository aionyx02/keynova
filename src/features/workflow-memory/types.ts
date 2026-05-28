// REF.5 — Frontend twin of `src-tauri/src/core/knowledge_store.rs::WorkflowHistoryRow`
// and the response shape returned by `workflow.recent` / `workflow.suggest`.
//
// Schema is additive-only per ADR-0029 §4 evolution rule. New fields are
// optional; unknown fields are ignored at parse time.

/** One workflow_history row, server-stamped id + executed_at. */
export interface WorkflowHistoryRow {
  id: number;
  /** ADR-0029 §4 coarse context: hash(workspace_id, mode, panel). */
  context_hash?: string | null;
  /** Backend route that produced the event. */
  route: "action.run" | "cmd.run" | "capability.call" | string;
  /** Human-readable label (action.label / cmd name / capability id). */
  action_label: string;
  /** Optional deterministic payload digest for v2 repeat detection. */
  payload_digest?: string | null;
  workspace_id?: number | null;
  /** Unix epoch seconds (server-stamped at INSERT). */
  executed_at: number;
}

/** Response payload from `workflow.recent` / `workflow.suggest`. */
export interface WorkflowSuggestResponse {
  rows: WorkflowHistoryRow[];
}
