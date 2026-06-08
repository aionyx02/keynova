// PROFILE.2 (ADR-0055): per-workspace pinned commands surfaced atop the computed
// `profile` list. Pure helpers — the backend stores the pin strings; the merge,
// dedupe, and pin↔suggestion mapping are a client-side concern over the
// untouched `workspace_profile` capability output.

import type { SuggestedNextAction } from "./types";

/** Pinned rows always sort to the top with full confidence. */
const PINNED_CONFIDENCE = 1;

export interface PinLabels {
  rationale: string;
  subtitle: string;
}

/**
 * Canonical `/name args` command for a row, reconstructed from its `cmd.run`
 * replay descriptor. This is the exact string stored as a pin, so a computed
 * profile row and its pinned twin share one key (stable toggle + dedupe).
 * Returns `null` for non-replayable rows (those are not pinnable).
 */
export function commandKeyOf(item: Pick<SuggestedNextAction, "replay">): string | null {
  const replay = item.replay;
  if (!replay || replay.route !== "cmd.run") return null;
  const name = typeof replay.payload.name === "string" ? replay.payload.name.trim() : "";
  if (!name) return null;
  const args = typeof replay.payload.args === "string" ? replay.payload.args.trim() : "";
  return args ? `/${name} ${args}` : `/${name}`;
}

/**
 * Build a displayable suggestion from a stored pin string, mirroring the
 * backend `replay_for_row`. Returns `null` for a pin that is not a `/name`
 * command (kept in storage, just not rendered).
 */
export function pinToSuggestion(pin: string, labels: PinLabels): SuggestedNextAction | null {
  const trimmed = pin.trim();
  if (!trimmed.startsWith("/")) return null;
  const body = trimmed.slice(1);
  const space = body.indexOf(" ");
  const name = (space === -1 ? body : body.slice(0, space)).trim();
  if (!name) return null;
  const args = space === -1 ? "" : body.slice(space + 1).trim();
  return {
    title: trimmed,
    subtitle: labels.subtitle,
    route: "cmd.run",
    confidence: PINNED_CONFIDENCE,
    rationale: labels.rationale,
    last_executed_at: 0,
    workspace_id: null,
    replay: { route: "cmd.run", payload: { name, args } },
  };
}

/**
 * Prepend pinned commands to the computed profile, dropping any computed row
 * that duplicates a pin so each pinned command appears once, at the top.
 */
export function mergeProfileWithPins(
  pins: string[],
  profile: SuggestedNextAction[],
  labels: PinLabels,
): SuggestedNextAction[] {
  if (pins.length === 0) return profile;
  const pinSet = new Set(pins);
  const pinned = pins
    .map((pin) => pinToSuggestion(pin, labels))
    .filter((item): item is SuggestedNextAction => item !== null);
  const rest = profile.filter((item) => {
    const key = commandKeyOf(item);
    return key === null || !pinSet.has(key);
  });
  return [...pinned, ...rest];
}
