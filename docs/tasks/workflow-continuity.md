---
type: task_plan
status: active
priority: p1
context_policy: on_demand
owner: project
tags: [workflow, continuity, suggest-next, window-stability, ux]
---

# CONT - Workflow Continuity + Stable Surface

Branch: `feature/workflow-continuity` (from `main`). Developer-directed
functional optimization. Two felt problems share one root — **the workflow does
not flow**: you cannot do one action then fluidly the next, and the window
visibly jitters as content changes. Plan file:
`~/.claude/plans/cuddly-pondering-haven.md`.

Primary = CONT.1 + CONT.2. CONT.3 is parked (simplification-only, non-blocking).

## CONT.1 - Predictive, proactive `next` (primary)

Today `suggest_next` only re-ranks the recency tail of `workflow_history` by
recency/route/context (`capabilities/suggest_next.rs::rank_rows_at`,
`workflow_memory::suggest` → `recent_workflows_blocking`). No prediction,
cold-start empty, reachable only via the `next` prefix, and nothing tees up a
follow-up after an action runs.

Scope:

- Backend: add **transition-aware** ranking. From rows ordered by `executed_at`
  within the active `context_hash`/`workspace_id`, build `(prev → next)`
  frequency pairs; given the most recent action (an **anchor**), rank candidate
  next actions by `P(next | anchor)` blended with recency + frequency. Fall back
  to the current recency tail when there is no anchor or history is sparse.
  Reuse `WorkflowHistoryRow`. Keep the ranker a pure fn (testable like
  `rank_rows_at`). Thread the anchor through `SuggestNextCtx` / `CapabilityRequest`.
  Preserve `is_stale` / dedupe / `target_resolves` guards.
- Frontend: surface top predictions **proactively** in the empty/idle state
  (palette open, no query) via `useSuggestNext` + `CapabilityListCard` +
  existing `runSuggestedWorkflow` replay (cmd.run only, no new exec path). Re-run
  `suggest_next` after a command/action completes (`useExecCommand`) so the
  surface tees up the next step. Keep the `next` prefix working.
- Governance: ranking-algorithm change → ADR-0052 (proposed).

Done:

- After running a command, reopening shows a relevant predicted next step that
  tracks the last action; sparse history degrades to the recency tail.
- Pure transition-ranking unit tests (anchor A → predicts following B; fallback).

## CONT.2 - Stable window (primary)

`src/hooks/useWindowResize.ts` fires a native `setSize()` on every DOM mutation
(ResizeObserver + MutationObserver), so streaming tokens and result population
make the OS window chase content height and jitter; width toggle recenters →
horizontal jump; `window.rs::show_launcher_window` shows before the frontend
sizes → open jump.

Scope:

- Decouple the OS window from variable content height: stable max height +
  internal scroll for the result list / capability cards (extend the existing
  scroll region); only `setSize` for structural modes (terminal already special).
- Coalesce + threshold remaining resizes (ignore sub-threshold deltas, trailing
  settle, no shrink-then-grow within one exchange).
- Smooth open: size + position before `show()`.
- Keep narrow↔wide as one deliberate transition, not per-keystroke.

Done:

- Ctrl+K opens in place (no show-then-jump); streaming an `explain` keeps the
  window steady (internal scroll); narrow↔wide is one clean transition.

## CONT.3 - Keystroke smoothness (parked, simplification-only)

Not blocking. If "flow" still feels off after CONT.1/.2: trim the 200ms
`SEARCH_DEBOUNCE_MS` and memoize the `useQueryChange` deps object.

## Non-goals

- No AI model/latency work (CPU inference wall is the known REF.7.D limit).
- No execution path for generated commands; suggestions stay copy/replay-only.
- Do not remove the `next` prefix.

## Status (2026-06-07)

- CONT.1 backend **done**: transition-aware ranking + anchor + fallback, ADR-0052
  proposed, +2 tests (534 Rust pass). Predictions labeled "likely next".
- CONT.1 proactive frontend surface **remaining**: idle-state predictions need
  the palette keyboard-nav state machine extended + `tauri dev` verification;
  deferred to a focused follow-up. The `next` prefix shows the new predictions
  today.
- CONT.2 **done** (logic): `useWindowResize` coalesces — grow-now / shrink-settle
  / sub-threshold-skip. Needs `tauri dev` confirmation of feel. Window pre-show
  reposition assessed, not changed (position persists across hide/show).
- CONT.3 parked.

Detail: `docs/memory/sessions/2026-06-07.md`.

## Validation

```bash
npm run lint && npm run test && npm run build
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
npm run docs:refresh
```

Manual (`npm run tauri dev`): open jump gone; window steady during streaming;
run-then-reopen shows a replayable predicted next step.
