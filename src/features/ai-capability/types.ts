// REF.4 — Frontend twin of `src-tauri/src/core/ai_capability/contract.rs`.
//
// Schema is additive only per ADR-0030 §4. Unknown fields are ignored on
// parse; missing fields fail safe (see parseRiskTagFailSafe).

import type { RiskTag } from "../../types/unified-result";

export type CapabilityId = "explain" | "summarize" | "fix_error";

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
