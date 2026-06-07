---
type: adr
status: proposed
priority: p1
updated: 2026-06-07
context_policy: retrieve_only
owner: project
---

# Workspace Command Profile (Project-Keyed)

**Status:** proposed
**Date:** 2026-06-07
**Decision makers:** AI agent draft; developer acceptance requested by PROFILE.1
**Related documents:**

- `docs/tasks/product-4-profiles.md` (§PROFILE.1)
- `docs/adr/0052-transition-aware-workflow-suggestion.md`
- `docs/adr/0053-workflow-outcome-success-rate.md`
- `docs/adr/0029-ai-capability-layer.md` (§4 workflow memory)

---

## 1. Context

PRODUCT.4 wants per-project "workspace profiles": the signature commands you use
in *this* project. `workflow_history` already carries `workspace_id` (the manual
slot 0/1/2) and `succeeded` (ADR-0053), but a slot is not a project — the same
slot is reused for different projects over time, so slot-keyed history cannot
form a stable project profile. The `WorkspaceState` does know its `project_root`,
but that root is never written to history.

## 2. Decision

Key workspace profiles by **`project_root`** and surface a project's signature
commands as a new copy/replay-only capability.

- **Schema (v6 → v7).** Add a nullable `project_root TEXT` column to
  `workflow_history`: `CREATE TABLE` for fresh DBs + an idempotent
  `ALTER TABLE ... ADD COLUMN` guarded by `PRAGMA table_info` for existing DBs
  (the same migration shape as ADR-0053's `succeeded`). `WorkflowHistoryEntry`
  gains `project_root: Option<String>`; `record_workflow_event` fills it from the
  current workspace's `project_root`. Additive, backward-compatible; pre-migration
  backup already covers rollback.
- **Profile ranking.** A new `workspace_profile` capability returns the current
  project's top commands, ranked by **frequency × success rate** over rows whose
  `project_root` matches the active workspace — reusing the existing
  `build_frequencies` / `build_success_stats` helpers. It deliberately omits the
  transition/anchor model (that is `next`/ADR-0052); a profile is "your toolkit
  here", not "your next step". **Cold-start fallback:** scope by `project_root`
  when it has rows, else by `workspace_id` (the active slot), else the global
  recency tail — so an existing user with `NULL`-project history still gets a
  useful profile while project-tagged rows accrue.
- **Surface.** Reuse the `CapabilityListCard` + `runSuggestedWorkflow` replay
  (copy/replay-only, `cmd.run` only — no new execution surface). Entry is
  **on-demand** via a `profile` prefix — deliberately *not* auto-shown in the
  idle state, so it stays distinct from the automatic idle `next` surface.

Keying by `project_root` (not slot) is the decision: it makes the profile follow
the project across slot reuse, at the cost of one additive column.

Rejected alternatives:

- *Slot-keyed profile (no schema).* Cheaper, but a profile would scramble when a
  slot is reused for a different project — the exact failure this closes.
- *Reuse `suggest_next` with a mode flag.* Conflates "next step" and "project
  toolkit"; a separate capability keeps each ranking and its tests clean.

## 3. Consequences

- One additive local-schema column; no IPC contract or public data change beyond
  the new capability. The `action_label` redaction/sanitize path is unchanged.
- Profiles only populate from new history (rows recorded after upgrade carry
  `project_root`); the global fallback covers the cold-start window.
- A reliably-failing command is already down-ranked/dropped by ADR-0053, so the
  profile inherits that safety.

## 4. Rollback

Drop the `workspace_profile` capability and stop writing `project_root`; the
column can remain unused. No data loss — history still ranks by
recency/transition/frequency/workspace/success.
