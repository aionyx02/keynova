// Frontend twin of `src-tauri/src/core/ai_capability/contract.rs`.
//
// Schema is additive only per ADR-0030 §4. Unknown fields are ignored on
// parse; missing fields fail safe (see parseRiskTagFailSafe).

import type { RiskTag } from "../../types/unified-result";
import type { ActionRef } from "../../types/search";

export type CapabilityId = "explain" | "summarize" | "fix_error" | "gen_command" | "suggest_next";

export interface CapabilityMeta {
  id: CapabilityId;
  audit: boolean;
  accepts_context_hash: boolean;
}

export type CapabilityOutput =
  | { kind: "text"; text: string }
  | { kind: "structured"; value: unknown };

/** Backend `capability.response` event payload. */
export interface CapabilityResponseEvent {
  request_id: string;
  ok: boolean;
  /** Present when `ok === true`. */
  id?: CapabilityId;
  output?: CapabilityOutput;
  risk_tag?: RiskTag;
  /** Present when `ok === false`. */
  error?: string;
  cancelled?: boolean;
}

/** Backend `capability.stream.chunk` event payload. */
export interface CapabilityStreamChunkEvent {
  request_id: string;
  delta: string;
}

/** Payload accepted by the `explain` capability. */
export interface ExplainPayload {
  text: string;
  question?: string;
}

/** Payload accepted by the `summarize` capability. */
export interface SummarizePayload {
  text: string;
  max_sentences?: number;
}

/** Payload accepted by the `fix_error` capability. v1 is explanation-only. */
export type FixErrorPayload =
  | { raw_output: string }
  | { program: string; args: string[]; cwd: string; timeout_secs?: number };

/** Optional execution context for `gen_command`. */
export interface GenCommandCtx {
  cwd?: string;
  shell?: string;
  os?: string;
}

/** Payload accepted by the `gen_command` capability. */
export interface GenCommandPayload {
  intent: string;
  ctx?: GenCommandCtx;
}

/** Structured reply returned by `gen_command`. */
export interface GenCommandOutput {
  command: string;
  confidence: number;
  rationale: string;
}

/** Optional workflow-memory query context for `suggest_next`. */
export interface SuggestNextCtx {
  limit?: number;
}

/** Payload accepted by the `suggest_next` capability. */
export interface SuggestNextPayload {
  ctx?: SuggestNextCtx;
}

/** Best-effort replay descriptor for suggestions that can be re-issued. */
export interface ReplayActionDescriptor {
  route: string;
  payload: Record<string, unknown>;
}

/** Structured row returned by the `suggest_next` capability. */
export interface SuggestedNextAction {
  title: string;
  subtitle: string;
  route: string;
  confidence: number;
  rationale: string;
  last_executed_at: number;
  workspace_id?: number | null;
  replay?: ReplayActionDescriptor | null;
  action_ref?: ActionRef;
}

function parseReplayActionDescriptor(value: unknown): ReplayActionDescriptor | null {
  if (!value || typeof value !== "object") {
    return null;
  }
  const candidate = value as Record<string, unknown>;
  if (typeof candidate.route !== "string") {
    return null;
  }
  if (!candidate.payload || typeof candidate.payload !== "object") {
    return null;
  }
  return {
    route: candidate.route,
    payload: candidate.payload as Record<string, unknown>,
  };
}

function parseSuggestedNextAction(value: unknown): SuggestedNextAction | null {
  if (!value || typeof value !== "object") {
    return null;
  }
  const candidate = value as Record<string, unknown>;
  if (
    typeof candidate.title !== "string" ||
    typeof candidate.subtitle !== "string" ||
    typeof candidate.route !== "string" ||
    typeof candidate.confidence !== "number" ||
    typeof candidate.rationale !== "string" ||
    typeof candidate.last_executed_at !== "number"
  ) {
    return null;
  }
  return {
    title: candidate.title,
    subtitle: candidate.subtitle,
    route: candidate.route,
    confidence: candidate.confidence,
    rationale: candidate.rationale,
    last_executed_at: candidate.last_executed_at,
    workspace_id: typeof candidate.workspace_id === "number" ? candidate.workspace_id : null,
    replay: parseReplayActionDescriptor(candidate.replay),
    action_ref: undefined,
  };
}

export function parseGenCommandOutput(value: unknown): GenCommandOutput | null {
  const structured =
    value && typeof value === "object" && "kind" in value ? (value as CapabilityOutput) : null;
  const candidate =
    structured?.kind === "structured" && structured.value && typeof structured.value === "object"
      ? (structured.value as Record<string, unknown>)
      : value && typeof value === "object"
        ? (value as Record<string, unknown>)
        : null;
  if (
    !candidate ||
    typeof candidate.command !== "string" ||
    typeof candidate.confidence !== "number" ||
    typeof candidate.rationale !== "string"
  ) {
    return null;
  }
  return {
    command: candidate.command,
    confidence: candidate.confidence,
    rationale: candidate.rationale,
  };
}

export function parseSuggestNextOutput(value: unknown): SuggestedNextAction[] {
  const structured =
    value && typeof value === "object" && "kind" in value ? (value as CapabilityOutput) : null;
  const candidate = structured?.kind === "structured" ? structured.value : value;
  if (!Array.isArray(candidate)) {
    return [];
  }
  return candidate
    .map((item) => parseSuggestedNextAction(item))
    .filter((item): item is SuggestedNextAction => item !== null);
}
