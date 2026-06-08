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
  helpers — `build_frequencies`, `build_success_stats`). Copy/replay-only,
  `cmd.run` only. Distinct ranking from `next`: no transition/anchor; pure project
  habit + reliability.
  - **Cold-start fallback (refinement):** scope by `project_root` when it has
    rows, else by `workspace_id` (the active slot), else the global recency tail.
    So the profile is useful from day one while project-tagged history accrues.
- **Surface (refinement):** **on-demand only** — a `profile` prefix (new
  `CapabilityId::WorkspaceProfile` + registry/contract wiring) that reuses
  `CapabilityListCard` + `runSuggestedWorkflow`. **Not** auto-shown in the idle
  state, so it stays distinct from the idle `next` surface; `next` remains the
  automatic one, `profile` is "show my toolkit here" when asked. Pure ranker
  stays unit-tested.

Done:

- Opening the palette in a project shows that project's signature commands,
  reliability-weighted; switching project changes the profile; replay works from
  the keyboard. New unit tests for project-scoped ranking + legacy handling.

## PROFILE.2 - Per-workspace pinned commands (done, ADR-0055)

Let a workspace carry a curated `pinned_commands` set surfaced atop the computed
profile, so a command you rely on is always one keystroke away even if it drifts
off the frequency-ranked list.

- **Storage** (`workspaces.json` v2→v3): `WorkspaceState.pinned_commands:
  Vec<String>` (`#[serde(default)]`, additive), capped at `MAX_PINS = 8`,
  per-slot. `WorkspaceManager::toggle_pin` adds/removes (most-recent first).
- **IPC**: additive `workspace.pin { command }` toggles + returns the new set;
  `workspace.get_current` already exposes `pinned_commands`.
- **Surface**: pins merged atop the `profile` list client-side
  (`workspacePins.ts` — pure `pinToSuggestion` / `mergeProfileWithPins` /
  `commandKeyOf`, dedupes the computed twin). `useWorkspacePins` hook loads/toggles.
  Pin/unpin via a clickable 📌 on each pinnable (`cmd.run`) row **or** Ctrl+P on
  the selected row; a first-run onboarding banner shows until the workspace has a
  pin, plus a footer hint. `workspace_profile` capability untouched (no ranker
  change). Copy/replay-only.

Done: pin a profile command, it sticks atop the list with 📌; toggle removes it;
pins are per slot and survive the command leaving the computed profile. Unit
tests: `workspace_manager` toggle/cap, `workspacePins` round-trip/merge,
`useKeyboardNav` Ctrl+P gating.

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
