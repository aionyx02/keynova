---
type: task_index
status: active
priority: p0
updated: 2026-05-16
context_policy: always_retrievable
owner: project
tags: [feature-first, safety-first, performance, agent-ux]
last_change: Phase 7a (AGENT.1/2/7) delivered — streaming, markdown, cancel, autosize, history, FIFO archive, approval timeout
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

- TD.5.A baseline → PERF.1 → TD.1/2/3 → PERF.2 → PERF.3 → TD.4 + TD.5 → P3 → FEAT.11 → **Phase 7a (AGENT.1/2/7)**.

Active queue：**empty**（待使用者選定下一階段；建議 Phase 8a UTIL.1/2 / Phase 8b LAUNCH.1 / Phase 8c ONBOARD.1，皆無 ADR 阻擋）。

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

- 2026-05-16: Phase 7a Agent / AI panel UX upgrade complete — AGENT.1.A–E + AGENT.2.A–C + AGENT.7.A/B/C/D (12 sub-items, 9 slices):
  - **Slice 1**: `src/ipc/routes.ts` — AI/Agent route constants (`AI_CHAT`, `AI_CANCEL`, `AGENT_START`, …); `useAi.ts` + `useAgent.ts` migrated off literal strings; `AiHandler.cancel_flags: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>` scaffolded.
  - **Slice 2 (AGENT.1.C 後端)**: `ai.cancel` IPC route; `chat_async(cancel_flag, cancel_registry)` short-circuits before/after `do_chat`, rolls back the user history entry on cancel, emits `ai.response { cancelled:true }`, and self-cleans the registry on completion.
  - **Slice 3 (AGENT.1.A 後端)**: SSE/NDJSON streaming for Ollama / OpenAI / Claude; each chunk emits `ai.stream.chunk`; new `ai.stream_enabled` setting (default `true`); pure parsers (`parse_ollama_chunk`/`parse_openai_chunk`/`parse_claude_chunk`) + `accumulate_stream` accumulator unit-tested.
  - **Slice 4 (AGENT.1.A/C/E 前端)**: `useAi.ts` listens for `ai-stream-chunk`, exposes `cancel()`; new `<ElapsedTimer>` ticks every 500 ms and surfaces a `Long running — Cancel?` hint at 30 s; Agent panel Cancel button now also shows while `running` / `planning`.
  - **Slice 5 (AGENT.1.B/D)**: `react-markdown` + `remark-gfm` + `rehype-highlight` added; new `<Markdown>` wrapper (raw HTML disabled, `<pre>` ErrorBoundary fallback); `src/utils/aiErrors.ts` classifier covers `connection_refused`/`model_not_found`/`unauthorized`/`timeout`/`oom`/`cancelled`; `<ErrorCard>` exposes Open-AI-settings / external-doc / Retry CTAs. Also fixed FEAT.11 typo: `useIPC` → `useIPCContext` in `LearningMaterialPanel.tsx`.
  - **Slice 6 (AGENT.2.A/B)**: Hover-revealed Copy / Regenerate per assistant + Agent final-answer card; `useTextareaAutosize` (rows 1–8); `useLocalHistory(storageKey, cap)` backs ↑/↓ recall against `keynova.chatHistory` / `keynova.agentHistory` (cap 50 each, dedupe).
  - **Slice 7 (AGENT.2.C)**: New `agent.show_audit_by_default` setting (default `false`); audit `<details>` honours it on mount.
  - **Slice 8 (AGENT.7.C)**: `AgentRuntime` now owns `RunStore { runs, order }` and an `AgentArchiveSink` trait; `insert_run` enforces FIFO cap (default `agent.run_history_cap = 20`), archives evicted runs through `KnowledgeStoreArchiveSink → agent_archive` table (new SQLite migration), and publishes `agent.run.archived`; frontend `useAgent.archivedCount` + AiPanel hint surface the eviction.
  - **Slice 9 (AGENT.7.A/B/D)**: `AgentApproval` gains `tool_name` / `deadline_unix_ms` / `remember_for_run` (`#[serde(default)]`); approval timeout configurable via `agent.approval_timeout_secs` (default 300); `wait_for_react_approval` now mutates the approval to `"approval_timeout"` and emits `agent.approval.timeout`; `agent.approve` accepts `remember: bool`; ReAct loop short-circuits the gate when an earlier approval for the same `tool_name` had `remember_for_run=true`. New `<ApprovalCard>` consolidates `<CountdownPill>` (60s amber / 10s red / "Timed out"), the **Approve and remember for this run** checkbox, an `<ApprovalSummary>` dispatcher across 8 `AgentActionKind`s, and a `Show raw` fallback.
  - Tests: 242/243 backend (1 pre-existing failure unchanged); new Slice-2 cancel tests, Slice-3 streaming parser + accumulator tests, Slice-8 FIFO eviction test, Slice-9 approval-timeout test.
  - Settings additions: `ai.stream_enabled`, `agent.show_audit_by_default`, `agent.run_history_cap`, `agent.approval_timeout_secs`.
  - Lint / tsc / clippy `-D warnings` all green.

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
