---
type: task_plan
status: active
priority: p1
context_policy: on_demand
owner: project
tags: [product-4, workflow-memory, ranking, replay, suggest-next]
---

# PRODUCT.4 - Daily-Use Workflow: ranking + replay

Branch: `feature/product-4-ranking` (from `main`). Developer-directed forward
work, continuing the CONT.1 transition-aware `suggest_next` ranking (ADR-0052).
PRODUCT.4 goal (roadmap): make recent successful workflows visibly influence
search and `next`, with inspectable, risk-gated replay.

`workflow_history` today records *attempts* only — columns: `context_hash`,
`route`, `action_label`, `payload_digest`, `workspace_id`, `executed_at`. There
is no success/outcome column, so success-rate ranking needs a schema change and
is split out (PRODUCT.4.B).

## PRODUCT.4.A - Frequency + workspace-affinity ranking (primary, no schema change)

Extend `suggest_next` ranking (`capabilities/suggest_next.rs::rank_rows_at`) with
two signals derived entirely from the existing history window CONT.1 already
pulls — no schema, no IPC, no new ADR (refinement of ADR-0052):

- **Frequency**: count occurrences of each candidate `(route, action_label)` in
  the window; add a bounded, normalized frequency bonus so habitually-repeated
  actions rank up. Avoids letting a single very-frequent action dominate (log or
  capped normalization).
- **Workspace affinity**: boost candidates whose `workspace_id` matches the
  **anchor** row's `workspace_id` (the most-recent action, already in the
  window) — "what I do *in this project*" — without threading a new request
  field. Distinct from `same_context` (exact workspace+mode+panel hash); this is
  a looser same-workspace signal.
- Surface the new signals in the row subtitle/rationale (e.g. "used N×") so the
  card stays explainable.

Keep the ranker a pure function; preserve `is_stale` / dedupe / `target_resolves`
guards and the replay-first sort. Blend: `confidence = recency/route/context +
transition·w + frequency·w + workspace·w − echo`, clamped.

Done:

- Frequently-repeated and same-workspace actions rank above one-off, other-
  workspace actions of equal recency; cold-start still degrades to recency.
- New pure unit tests for frequency and workspace-affinity ordering; existing
  `suggest_next` tests stay green.

## PRODUCT.4.B - Success-rate signal (done, ADR-0053)

`workflow_history` gained a nullable `succeeded` column (schema v5→6; idempotent
`ALTER TABLE` for existing DBs). `cmd_dispatch_impl` now records `cmd.run` /
`capability.call` in **both** the Ok and Err branches, and `run_action_command`
records every attempt (was success-only) with its outcome. `suggest_next` adds a
`(success_rate − 1.0)·SUCCESS_WEIGHT` term (perfect record = neutral baseline so
it can't saturate; failures penalize) and **drops** a candidate with ≥3 attempts
and zero successes. Legacy `NULL` rows count as success. +2 pure unit tests.

## PRODUCT.4.C - Replay provenance (small follow-up)

CONT.1 already replays `cmd.run` suggestions (copy/replay-only, `runSuggestedWorkflow`).
PRODUCT.4 wants "clear provenance": confirm the replay row shows source +
last-run + risk clearly (the idle card already renders rationale + executed-at).
Polish only; assess after 4.A.

## Non-goals

- No success/outcome schema change in this branch (that is 4.B).
- No generated-command execution path; replay stays the existing safe `cmd.run`.
- No workspace-profiles feature (separate PRODUCT.4 item).

## Validation

```bash
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
npm run lint && npm run test && npm run build
npm run docs:refresh
```
