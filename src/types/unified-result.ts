// REF.1 — Frontend twin of `src-tauri/src/models/unified_result.rs`.
//
// Shape must stay in sync with the Rust side. Schema evolution is additive only
// per ADR-0030 §4; new fields go in as optional. REF.6 will switch the palette
// to consume these types.

import type { ActionRef, ResultKind, ScoreBreakdown } from "./search";

/** ADR-0030 minimal risk tag. `reason` is audit-only — UI must not render it. */
export interface RiskTag {
  requires_confirmation: boolean;
  /** Audit-only; never displayed to users. Capability end truncates to 256 UTF-8 bytes. */
  reason?: string;
}

/** REF.1 alias for RiskTag. Use either name; they are the same shape. */
export type ConfirmRequirement = RiskTag;

/** ADR-0030 §4 fail-safe: unknown / malformed risk tag → require confirmation. */
export function parseRiskTagFailSafe(value: unknown): RiskTag {
  if (value && typeof value === "object" && "requires_confirmation" in value) {
    const v = value as Record<string, unknown>;
    const flag = v.requires_confirmation;
    if (typeof flag === "boolean") {
      return {
        requires_confirmation: flag,
        reason: typeof v.reason === "string" ? v.reason : "",
      };
    }
  }
  return { requires_confirmation: true, reason: "" };
}

/** Where a result came from. Tagged union — serde tag `type` on the Rust side. */
export type ResultSource =
  | { type: "file"; kind: ResultKind; path: string }
  | { type: "builtin_command"; ui: BuiltinUi }
  | { type: "other"; name: string };

/** Flattened builtin command UI hint. Drops `TerminalLaunchSpec` payload by design. */
export type BuiltinUi =
  | { type: "inline" }
  | { type: "panel"; name: string }
  | { type: "terminal" };

/** An action exposed inline on a result row. */
export interface ActionChip {
  id: string;
  label: string;
  action_ref: ActionRef;
  confirm: ConfirmRequirement;
  hotkey_hint?: string;
  /** True = visible on the row; false = inside the secondary menu. */
  primary?: boolean;
}

/** Preview payload, rendered on user expand gesture. Tagged union — serde tag `kind`. */
export type PreviewPayload =
  | { kind: "none" }
  | { kind: "text"; content: string; truncated: boolean }
  | { kind: "image"; data_url: string }
  | { kind: "binary"; size_bytes: number };

/** Ranking signals. `workflow_boost` stays 0 until REF.5 wires workflow memory. */
export interface RankSignals {
  score: number;
  breakdown?: ScoreBreakdown;
  workflow_boost?: number;
}

/** Forward-compat metadata bag — never put required semantics here. */
export interface SourceMetadata {
  tags?: string[];
  modified_ms?: number;
  size_bytes?: number;
}

/** Unified result/action contract (REF.1). */
export interface UnifiedResult {
  id: string;
  source: ResultSource;
  title: string;
  subtitle: string;
  icon_key?: string;
  actions: ActionChip[];
  preview?: PreviewPayload;
  rank?: RankSignals;
  /** Optional workflow-memory context hash (REF.5). */
  context_hash?: string;
  source_metadata?: SourceMetadata;
}

/** Build a no-confirm RiskTag (matches Rust `RiskTag::none()`). */
export function riskTagNone(): RiskTag {
  return { requires_confirmation: false, reason: "" };
}
