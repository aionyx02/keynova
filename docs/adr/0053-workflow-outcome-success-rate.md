---
type: adr
status: proposed
priority: p1
updated: 2026-06-07
context_policy: retrieve_only
owner: project
---

# Workflow Outcome + Success-Rate Ranking

**Status:** proposed
**Date:** 2026-06-07
**Decision makers:** AI agent draft; developer acceptance requested by PRODUCT.4.B
**Related documents:**

- `docs/tasks/product-4-ranking.md` (§PRODUCT.4.B)
- `docs/adr/0052-transition-aware-workflow-suggestion.md`
- `docs/adr/0029-ai-capability-layer.md` (§4 workflow memory)

---

## 1. Context

PRODUCT.4 wants `suggest_next` ranking to reflect recent **successful** workflows.
But `workflow_history` records **attempts on success only** today — `action.run`
records inside `run_action_command` guarded by `result.is_ok()`, and the central
`cmd.run` / `capability.call` hook records only in the `Ok` branch of
`cmd_dispatch_impl`. There is no failure signal and no `succeeded` column, so a
success rate cannot be computed, and a command that reliably fails is invisible
rather than down-ranked.

## 2. Decision

Record the **outcome** of each workflow event and fold a success-rate signal into
ranking.

- **Schema (v5 → v6).** Add a nullable `succeeded INTEGER` column to
  `workflow_history` (`1` success, `0` failure, `NULL` for legacy rows). Fresh
  DBs get it via `CREATE TABLE`; existing DBs via an idempotent
  `ALTER TABLE ... ADD COLUMN` (added only when absent, checked via
  `PRAGMA table_info`). Additive and backward-compatible; the pre-migration
  backup path already covers rollback.
- **Recording.** `WorkflowHistoryEntry` gains `succeeded: Option<bool>`.
  `cmd_dispatch_impl` records `cmd.run` / `capability.call` in **both** the `Ok`
  and `Err` branches with the real outcome; `run_action_command` records every
  attempt (not just successes) with `succeeded = result.is_ok()`. Recording stays
  fire-and-forget and best-effort.
- **Ranking.** `suggest_next` computes a per-action success rate over the window
  (legacy `NULL` counts as success, since only successes were recorded before),
  adds a `(success_rate − 1.0) · SUCCESS_WEIGHT` term to the blend — a perfect
  record is the neutral baseline (no bonus, so it can't saturate the score) and
  only failures penalize — and **drops a candidate that has clearly broken** (≥3
  attempts, zero successes) so reliably-failing actions are not suggested. Pure
  ranker stays unit-tested.

Legacy rows are treated as successes, so existing behavior is preserved on
upgrade; the signal only differentiates once new outcome data accrues.

Rejected alternatives:

- *Join `action_logs` (which already has ok/error) at rank time.* It is keyed by
  `action_id`, not the `route::label` the ranker dedupes on, and does not cover
  `cmd.run` / `capability.call`; a single `succeeded` column on the same table is
  simpler and uniform.
- *Keep success-only recording.* Cannot compute a rate; the exact gap PRODUCT.4.B
  closes.

## 3. Consequences

- Failed workflows now create history rows (down-ranked / filtered), so the table
  grows slightly faster; the existing recency/age pruning in `suggest_next`
  bounds the suggestion set regardless.
- A reliably-failing command stops being suggested; a flaky one ranks lower.
- One additive local-schema column; no IPC or public-contract change. The
  redaction/sanitize path for `action_label` is unchanged.

## 4. Rollback

Stop writing `succeeded` and drop the success-rate term + broken-candidate
filter; the column can remain unused (harmless) or be dropped. No data loss —
legacy and new rows still rank by recency/transition/frequency/workspace.
