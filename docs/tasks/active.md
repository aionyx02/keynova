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

Mainline delivered (詳見 `docs/tasks/completed.md`)：

- TD.5.A baseline → PERF.1 → TD.1/2/3 → PERF.2 → PERF.3 → TD.4 + TD.5 → P3 → FEAT.11.

Active queue：**empty**（待使用者於 Next Phase Proposal 中選定起手 track）。

## Next Phase Proposal (2026-05-15, awaiting selection)

Source: agent UX + 功能實用性 + keynova 整體 baseline 缺口盤點。

Recommended parallel entry tracks (highest ROI, no ADR blockers):

- **Phase 7a — AGENT.1 / AGENT.2 / AGENT.7**：streaming、markdown、cancel、approval UX。立即改善等待感與訊息可操作性。
- **Phase 8a — UTIL.1 / UTIL.2**：calculator++ 與 dev utilities。純 local、可平行於 agent 改造。
- **Phase 8b — LAUNCH.1**：search 結果 secondary actions（reveal / copy path / file ops）。放大既有 search 投資。
- **Phase 8c — ONBOARD.1**：first-run tour、`?` cheatsheet、empty-state CTA。降低學習曲線。

ADR-gated tracks（需等 ADR 接受才能進實作，可先草擬 ADR）：

- AGENT.3 (ADR-029) — agent 工具集從「research helper」改為「action executor」
- AGENT.4 (ADR-030) — OS-level selection capture
- CLIP.1 (ADR-031) — clipboard history
- SNIP.1 (ADR-032) — text expansion
- WIN.1 (ADR-033) — window switcher
- UTIL.3 (ADR-034) — scheduler / reminder
- DEV.1.C/D (ADR-035) — external provider auth
- SYNC.1.D/E/F (ADR-036) — git-backed sync
- AI.1 (ADR-037) — inline AI surfaces

See `docs/tasks/backlog.md` Post-FEAT.11 Phase Proposal 與 11 個 track section（總計約 80 個子任務）。
See `docs/tasks/blocked.md` Post-FEAT.11 Tracks Pending ADR 表。

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
