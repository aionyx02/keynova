---
type: task_plan
status: active
priority: p1
context_policy: on_demand
owner: project
tags: [product-4, workspace-profiles, workflow-memory, project-root]
---

# PRODUCT.4 - Workspace Profiles

Branch: `feature/product-4-profiles` (from `main`). Continues PRODUCT.4 after the
ranking work (4.A frequency+workspace, 4.B success-rate). Developer-directed;
profile key = **`project_root`** (confirmed 2026-06-07).

A *workspace profile* = "your signature commands + preferences for **this
project**" — answers "what do I usually do here" when the palette opens in a
project. Distinct from `next` (transition: "what comes after my last action").

## Current model (grounding)

- Workspaces are 3 manual slots (`workspaces.json`); each `WorkspaceState`
  already carries `project_root` (set at startup via `set_project_root_if_unset`).
- `workflow_history` records `workspace_id` (slot) + `succeeded` (ADR-0053) but
  **not** `project_root`, so commands are attributed to a *slot*, not a *project*.
- The idle/`next` `CapabilityListCard` + `runSuggestedWorkflow` replay exist and
  are reused here.

## PROFILE.1 - Project-scoped command profile (primary)

- **Schema** (`workflow_history` v6→7, ADR-0054): add a nullable `project_root`
  column — `CREATE TABLE` for fresh DBs + an idempotent `ALTER TABLE ... ADD
  COLUMN` guarded by `PRAGMA table_info` (same pattern as 4.B `succeeded`).
  `WorkflowHistoryEntry` gains `project_root: Option<String>`; INSERT/SELECT
  updated. `record_workflow_event` fills it from the current workspace's
  `project_root`.
- **Backend**: a `workspace_profile` capability returning the current project's
  top commands ranked by **frequency × success-rate** (reuse the 4.A/4.B ranker
  helpers — `build_frequencies`, `build_success_stats`), scoped to rows whose
  `project_root` matches the active workspace. Copy/replay-only, `cmd.run` only.
  Distinct ranking from `next`: no transition/anchor; pure project habit +
  reliability. Legacy `NULL` project_root rows are excluded from project scope
  (they predate the column) but a no-project fallback keeps the global tail.
- **Surface**: reuse `CapabilityListCard` + `runSuggestedWorkflow`. Entry via a
  `profile` prefix (new `CapabilityId::WorkspaceProfile` + registry/contract
  wiring) and/or the existing `WorkspaceIndicator`. Pure ranker stays unit-tested.

Done:

- Opening the palette in a project shows that project's signature commands,
  reliability-weighted; switching project changes the profile; replay works from
  the keyboard. New unit tests for project-scoped ranking + legacy handling.

## PROFILE.2 - Per-workspace preferences (deferred)

Let a workspace carry user-set preferences (pinned commands, default mode /
search backend). `WorkspaceState` already has `name`; add a small curated
`pinned_commands` set surfaced atop the profile. Config-shaped; its own slice.

## PROFILE.3 - Auto-activate on project change (deferred)

When the detected `project_root` changes, surface "you're in <project>" and load
the matching profile automatically. Larger UX; after PROFILE.1/.2.

## Non-goals

- No generated-command execution; replay stays the safe `cmd.run` path.
- No new workspace-slot mechanics; no preferences UI in PROFILE.1.
- No success-rate/transition rework (those are ADR-0052/0053).

## Validation

```bash
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
npm run lint && npm run test && npm run build
npm run docs:refresh
```

Manual (`npm run tauri dev`): open in project A → profile A; switch to project B
→ profile B; replay a profile command from the keyboard.
