---
type: adr
status: proposed
priority: p1
updated: 2026-06-07
context_policy: retrieve_only
owner: project
---

# Transition-Aware Workflow Suggestion (`next`)

**Status:** proposed
**Date:** 2026-06-07
**Decision makers:** AI agent draft; developer acceptance requested by CONT.1
**Related documents:**

- `docs/tasks/workflow-continuity.md` (§CONT.1)
- `docs/adr/0029-ai-capability-layer.md` (§4 workflow memory)
- `docs/architecture.md` (workflow_memory / ai_capability)

---

## 1. Context

The `suggest_next` capability (the `next` palette command) is meant to answer
"what would I likely do next?", but today it only returns the recency tail of
`workflow_history` filtered by `context_hash` (`workflow_memory::suggest` →
`recent_workflows_blocking`, ranked by `rank_rows_at` on recency/route/context).
That is a *recent-list*, not a *prediction*: it cannot say "after you run tests
you usually open the diff", it is empty until enough history accrues, and it is
only reachable by explicitly typing `next`. The result is a feature that feels
unfinished and a workflow that does not chain one action into the next.

## 2. Decision

Add a **transition-aware** ranking on top of the existing recency model, and
surface it **proactively**.

- **Model.** From `workflow_history` rows ordered by `executed_at` within the
  active `context_hash` / `workspace_id`, derive consecutive `(prev → next)`
  pairs and count their frequencies. Given an **anchor** (the user's most recent
  executed action), score each candidate next action as a blend of
  `P(next | anchor)` (transition frequency) with the existing recency + route +
  context signal. When there is no anchor, or transition support is too thin,
  fall back to the current recency-tail ranking — so behavior never regresses on
  cold start.
- **Plumbing.** Thread the anchor through `SuggestNextCtx` / `CapabilityRequest`
  as a new optional field. The ranking stays a **pure function** (the
  `rank_rows_at` pattern) so the transition logic is unit-tested without I/O.
  Existing `is_stale` / dedupe / `target_resolves` guards are preserved.
- **Surface.** Render the top predictions in the palette's empty/idle state and
  refresh them after an action completes, in addition to the existing `next`
  prefix (which stays). Suggestions remain **copy/replay-only** — replay reuses
  the existing safe `cmd.run` path; no new execution surface is introduced.

This is additive over schema v4 (`workflow_history` already carries
`executed_at`, `context_hash`, `workspace_id`, `route`, `action_label`); no
schema migration, no new IPC command, no network.

Rejected alternatives:

- *Keep the recency tail only.* Simplest, but it is the exact "feels unfinished"
  gap this ADR addresses.
- *A learned/embedding next-action model.* Over-engineered for local single-user
  history; a frequency bigram is interpretable, cheap, and offline.

## 3. Consequences

- `next` becomes predictive and proactive; the workflow chains more fluidly.
- Ranking is now order-sensitive, so tests must cover transition vs fallback
  paths. Suggestions are only as good as accumulated history (graceful cold-start
  fallback mitigates this).
- No data-format or security-boundary change; suggestions stay copy/replay-only,
  so risk posture is unchanged from the current `suggest_next`.

## 4. Rollback

Drop the anchor field and the transition branch; `suggest_next` reverts to the
recency-tail ranking. Remove the proactive empty-state surface to return to the
`next`-prefix-only entry. No persisted state is affected.
