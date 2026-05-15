---
type: task_index
status: active
priority: p0
updated: 2026-05-15
context_policy: always_retrievable
owner: project
tags: [feature-first, safety-first, performance]
last_change: feasibility review integrated from Keynova_Agent_Architecture_Tasks_Review.docx
---

# Active Tasks

## Feasibility Verdict (2026-05-14)

- Feasibility is high for low-memory runtime plus safe local review if rollout is staged.
- FEAT.11 runtime work remains blocked until ADR, security policy, and typed DTO prerequisites are complete.
- RAM goal is scoped to Background Core only (exclude active WebView, loaded LLM model, PTY terminal session, and monitoring streams).

## Strategy

Safety first for runtime lifecycle; feature-first delivery after guardrails are in place.

## Current Execution Order

1. TD.5.A minimal verify baseline as execution gate. ✓
2. PERF.1 Low Memory Background Mode. ✓
3. TD.1 + TD.2 + TD.3 prerequisite slices required by PERF and FEAT tracks. ✓
4. PERF.2 Search Execution Bound. ✓
5. PERF.3 Heavy Feature Lazy Runtime. ✓
6. TD.4 actor-like services and remaining TD.5 hardening. ✓
7. P3 Context Compiler Lite. ✓
8. FEAT.11 Learning Material Review. ✓

## Recent Execution Notes

- 2026-05-15: FEAT.11 Learning Material Review complete (FEAT.11.A–J):
  - `docs/adr/0028-learning-material-review-local-context.md`: ADR created (提議 status); workspace-root scope, symlink escape prevention, denylist, disabled-by-default.
  - `models/learning_material.rs`: `MaterialClass`, `MaterialCandidate`, `ScanStats`, `ReviewReport` with `to_markdown()`.
  - `managers/learning_material_manager.rs`: metadata-first scanner, `classify_by_extension`, `is_project_root`, `is_denied` (glob + exact), `preview_file` with secret redaction; DEFAULT_DENYLIST covers .env/.key/.pem/id_rsa/etc.
  - `handlers/learning_material.rs`: `scan`, `preview`, `export_note`, `export_markdown` IPC commands; canonicalized path write for markdown export.
  - `core/agent_runtime.rs`: `LearningMaterialReviewToolParams/Result` (JsonSchema); `learning_material.review` agent tool registered (ApprovalPolicy::Required, ActionRisk::Medium).
  - `handlers/agent/mod.rs`: `dispatch_learning_material_review()` wired into approval-gated dispatch.
  - 5 settings under `agent.local_context` (disabled by default); `default_config.toml` updated.
  - `src/components/LearningMaterialPanel.tsx`: full scan UI with stats bar, class filter tabs, candidate list, export-as-note action.
  - `src/ipc/routes.ts`: 3 new IPC route constants.
  - 13/13 FEAT.11 tests pass; 233/234 total (1 pre-existing failure unchanged); clippy clean.

- 2026-05-15: P3 Context Compiler Lite complete (P3.A + P3.B):
  - `models/context_bundle.rs`: `ContextBundle` struct with `WorkspaceContext`, `SelectedFileContext`, `ContextTokenBudget`; `build()` with 2-pass token budget.
  - `AgentHandler::build_context_bundle()`: manager-only assembly; no FS scan; bounded snippet preview (200 chars); 3000-char total cap.
  - `AgentRun.context_bundle: Option<ContextBundle>` (`#[serde(default)]`); both run paths wired.
  - Frontend types aligned in `useAgent.ts`.
  - 220/221 tests pass (1 pre-existing failure unchanged).
- 2026-05-15: AI panel UX hotfix delivered for clear behavior and chronology:
  - `AGENT` mode now has backend clear command (`agent.clear_runs`) and frontend clear wiring.
  - Agent runs are rendered in chronological order (old -> new) to keep the newest request at the bottom.
  - Chat history hydration now avoids stale overwrite after local send/clear actions.
- 2026-05-15: Setting panel tab bar layout adjusted for dense section lists:
  - replaced equal-width tab squeeze (`flex-1`) with content-width tabs.
  - enabled horizontal scrolling for section tabs to avoid overlap/truncation collisions.
  - added fallback title-casing for unmapped dynamic section labels.
- 2026-05-15: Setting panel tab visual polish pass:
  - tab rail now uses subtle dark gradients on both edges to match panel tone.
  - horizontal scrollbar themed to dark/slate and slimmed down to reduce visual noise.
  - tab typography tuned to surrounding UI scale (12px semibold, tighter spacing).

See `docs/tasks/backlog.md` for detailed breakdown.
See `docs/tasks/blocked.md` for gating constraints.
See `docs/tasks/completed.md` for P0/P1/P2 history.
