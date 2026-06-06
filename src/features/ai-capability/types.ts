// Frontend twin of `src-tauri/src/core/ai_capability/contract.rs`.
//
// Schema is additive only per ADR-0030 §4. Unknown fields are ignored on
// parse; missing fields fail safe (see parseRiskTagFailSafe).

import type { RiskTag } from "../../types/unified-result";
import type { ActionRef } from "../../types/search";

export type CapabilityId =
  | "explain"
  | "summarize"
  | "fix_error"
  | "gen_command"
  | "suggest_next"
  | "remember"
  | "recall";

export interface CapabilityMeta {
  id: CapabilityId;
  audit: boolean;
  accepts_context_hash: boolean;
}

export type CapabilityOutput =
  | { kind: "text"; text: string }
  | { kind: "structured"; value: unknown };

export interface CapabilitySource {
  source_id: string;
  source_type: string;
  title: string;
  uri?: string | null;
}

/** Backend `capability.response` event payload. */
export interface CapabilityResponseEvent {
  request_id: string;
  ok: boolean;
  /** Present when `ok === true`. */
  id?: CapabilityId;
  output?: CapabilityOutput;
  risk_tag?: RiskTag;
  sources?: CapabilitySource[];
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

/** Payload accepted by the copy-only `fix_error` capability. */
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

export interface CommandSuggestion {
  command: string;
  confidence: number;
  rationale: string;
}

/** Structured reply returned by `fix_error`. */
export interface FixErrorOutput {
  explanation: string;
  suggested_command?: CommandSuggestion | null;
}

/** Structured reply returned by `gen_command`. */
export interface GenCommandOutput extends CommandSuggestion {
  assumptions: GenCommandCtx;
}

/** Payload accepted by the `remember` capability. */
export interface RememberPayload {
  text: string;
}

/** Structured reply returned by `remember`. */
export interface RememberOutput {
  id: string;
  title: string;
  content: string;
  saved: boolean;
}

/** Payload accepted by the `recall` capability. */
export interface RecallPayload {
  query: string;
  limit?: number;
}

/** Structured row returned by the `recall` capability. */
export interface RecalledMemory {
  id: string;
  title: string;
  snippet: string;
  content: string;
  score: number;
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

function parseCommandSuggestion(value: unknown): CommandSuggestion | null {
  if (!value || typeof value !== "object") {
    return null;
  }
  const candidate = value as Record<string, unknown>;
  if (
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

function parseGenCommandCtx(value: unknown): GenCommandCtx {
  if (!value || typeof value !== "object") {
    return {};
  }
  const candidate = value as Record<string, unknown>;
  return {
    cwd: typeof candidate.cwd === "string" ? candidate.cwd : undefined,
    shell: typeof candidate.shell === "string" ? candidate.shell : undefined,
    os: typeof candidate.os === "string" ? candidate.os : undefined,
  };
}

export function parseCapabilitySources(value: unknown): CapabilitySource[] {
  if (!Array.isArray(value)) {
    return [];
  }
  return value.flatMap((item) => {
    if (!item || typeof item !== "object") {
      return [];
    }
    const candidate = item as Record<string, unknown>;
    if (
      typeof candidate.source_id !== "string" ||
      typeof candidate.source_type !== "string" ||
      typeof candidate.title !== "string"
    ) {
      return [];
    }
    return [
      {
        source_id: candidate.source_id,
        source_type: candidate.source_type,
        title: candidate.title,
        uri: typeof candidate.uri === "string" ? candidate.uri : null,
      },
    ];
  });
}

export function parseFixErrorOutput(value: unknown): FixErrorOutput | null {
  const structured =
    value && typeof value === "object" && "kind" in value ? (value as CapabilityOutput) : null;
  const candidate =
    structured?.kind === "structured" && structured.value && typeof structured.value === "object"
      ? (structured.value as Record<string, unknown>)
      : value && typeof value === "object"
        ? (value as Record<string, unknown>)
        : null;
  if (!candidate || typeof candidate.explanation !== "string") {
    return null;
  }
  return {
    explanation: candidate.explanation,
    suggested_command: parseCommandSuggestion(candidate.suggested_command),
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
  const suggestion = parseCommandSuggestion(candidate);
  if (!suggestion || !candidate) {
    return null;
  }
  return {
    ...suggestion,
    assumptions: parseGenCommandCtx(candidate.assumptions),
  };
}

export function parseRememberOutput(value: unknown): RememberOutput | null {
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
    typeof candidate.id !== "string" ||
    typeof candidate.title !== "string" ||
    typeof candidate.content !== "string" ||
    typeof candidate.saved !== "boolean"
  ) {
    return null;
  }
  return {
    id: candidate.id,
    title: candidate.title,
    content: candidate.content,
    saved: candidate.saved,
  };
}

function parseRecalledMemory(value: unknown): RecalledMemory | null {
  if (!value || typeof value !== "object") {
    return null;
  }
  const candidate = value as Record<string, unknown>;
  if (
    typeof candidate.id !== "string" ||
    typeof candidate.title !== "string" ||
    typeof candidate.snippet !== "string" ||
    typeof candidate.content !== "string" ||
    typeof candidate.score !== "number"
  ) {
    return null;
  }
  return {
    id: candidate.id,
    title: candidate.title,
    snippet: candidate.snippet,
    content: candidate.content,
    score: candidate.score,
  };
}

export function parseRecallOutput(value: unknown): RecalledMemory[] {
  const structured =
    value && typeof value === "object" && "kind" in value ? (value as CapabilityOutput) : null;
  const candidate = structured?.kind === "structured" ? structured.value : value;
  if (!Array.isArray(candidate)) {
    return [];
  }
  return candidate
    .map((item) => parseRecalledMemory(item))
    .filter((item): item is RecalledMemory => item !== null);
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
