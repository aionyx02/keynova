---
type: task_index
status: backlog
priority: p1
updated: 2026-05-15
context_policy: retrieve_when_planning
owner: project
tags: [feature-first, roadmap, performance, safety]
---

# Backlog Tasks

## Mainline Sequence

1. TD.5.A minimal verify baseline.
2. PERF.1 Low Memory Background Mode.
3. TD.1 + TD.2 + TD.3 prerequisite slices.
4. PERF.2 Search Execution Bound.
5. PERF.3 Heavy Feature Lazy Runtime.
6. TD.4 alignment + TD.5 hardening.
7. P3 Context Compiler Lite.
8. FEAT.11 Learning Material Review (blocked).

## PERF.1 - Low Memory Background Mode

Goal: make background runtime predictable and keep memory usage bounded.

- [x] PERF.1.A Define RAM budget boundary: Background Core < 100 MB and explicitly exclude active WebView, loaded LLM model, PTY terminal session, monitoring streams, and index rebuild tasks.
- [x] PERF.1.B Add `[performance].low_memory_mode = true` with settings read/write support. (`default_config.toml`, `settings_schema.rs`)
- [x] PERF.1.C Disable terminal prewarm in low memory mode. (`app/bootstrap.rs`)
- [x] PERF.1.D Move startup file indexing to lazy/manual/delayed policy. (`app/bootstrap.rs`)
- [x] PERF.1.E Add `ai.check_setup` TTL cache (300 s by provider/model/base URL). (`handlers/ai.rs`)
- [x] PERF.1.F Add `ai.ollama_keep_alive` policy, default `"5m"`, configurable to `"0s"`. (`ai_manager.rs`, `settings_schema.rs`, `default_config.toml`)
- [x] PERF.1.G Make model catalog refresh lazy or manual — already demand-driven via `model.recommend` command only; no startup trigger exists.
- [x] PERF.1.H LRU caps — `rank_memory` capped at 512 entries (pre-existing); `catalog_cache` bounded to top-12 Ollama library entries (pre-existing).
- [ ] PERF.1.I Add memory measurement checklist for cold start, idle tray, open palette, and agent run.

## Foundation TD Slices (Required Before FEAT.11 Runtime)

- [x] TD.1.A Centralize IPC client: replaced all `invoke("cmd_dispatch", ...)` calls in `CommandPalette` and `TerminalPanel` with `useIPC().dispatch()`; `cmd_keep_launcher_open` window command stays as a module-level helper.
- [x] TD.1.B Extract search domain logic to `src/utils/search.ts`: `searchResultKey`, `SOURCE_QUOTAS`, `applySourceQuotas`, `mergeSearchResults`, `sortSearchResults`, `searchSourceOrder`.
- [x] TD.1.C Split CommandPalette into focused hooks: `useWindowResize` (RAF + ResizeObserver/MutationObserver lifecycle), `useSearchMetadata` (lazy metadata + icon fetching).
- [x] TD.2.A Extract `AppContainer` composition root. (`src/components/AppContainer.tsx`, `src/context/IPCContext.tsx` — `IPCProvider` wraps stable dispatch; `App.tsx` renders `AppContainer`)
- [x] TD.2.B Add feature module registration boundary for lazy services. (`src/context/FeatureContext.tsx` — `FeatureProvider` + `useFeature()` with `activate(key)` interface)
- [x] TD.2.C Reduce `AppState::new()` startup assembly weight. (`src-tauri/src/app/state.rs` — `ManagerBundle`, `create_managers()`, `build_builtin_registry()`, `build_command_router()` extracted; `new()` reduced to ~15 lines)
- [x] TD.3.A Add typed request/response DTOs for critical routes. (`src-tauri/src/models/ipc_requests.rs` — DTOs for search.query, search.record_selection, setting.get/set, terminal.open/send/close/resize)
- [x] TD.3.B Reduce raw `Value` parsing in key handlers. (`handlers/setting.rs`, `handlers/terminal.rs`, `handlers/search.rs` — all critical commands use `serde_json::from_value::<TypedDTO>` instead of manual field access)
- [x] TD.3.C Align frontend route constants and payload types. (`src/ipc/routes.ts` — all IPC routes as `IPC.*` constants; `src/ipc/types.ts` — typed request/response interfaces; updated `useCommands`, `useHotkey`, `useSearchMetadata`, `CommandPalette`, `TerminalPanel`; `useCommands` migrated from direct `invoke` to `useIPC()`)

## PERF.2 - Search Execution Bound

Goal: remove unbounded thread growth and ensure cancellation works end-to-end.

- [ ] PERF.2.A Introduce SearchService coordinator as single request entry.
- [ ] PERF.2.B Replace per-query thread fan-out with bounded worker pool.
- [ ] PERF.2.C Add cancellation token propagation through indexer and fallback search.
- [ ] PERF.2.D Enforce latest-request-only behavior and discard stale responses.
- [ ] PERF.2.E Add backpressure for streaming chunks to avoid queue buildup.
- [ ] PERF.2.F Add regression tests for correctness, cancellation, timeout, and fallback.

## PERF.3 - Heavy Feature Lazy Runtime

Goal: keep heavy services off until user intent requires them.

- [ ] PERF.3.A Make TerminalManager lazy-init (no PTY/shell before first terminal use).
- [ ] PERF.3.B Make system monitoring stream session-scoped and stop-on-close.
- [ ] PERF.3.C Make Nvim manager lazy-init (no detect/download during startup).
- [ ] PERF.3.D Add AI Runtime Service for setup TTL, request queue, keep_alive, and unload policy.
- [ ] PERF.3.E Add Lazy Service/Feature Gate support in AppContainer.

## TD.4 + TD.5 Follow-Up

- [ ] TD.4.A Agent approval/cancel channelization cleanup.
- [ ] TD.4.B Merge SearchService actor changes with PERF.2 execution model.
- [ ] TD.4.C Merge Terminal actor boundary with PERF.3 lazy runtime model.
- [ ] TD.5.B Enforce CSP and network allowlist.
- [ ] TD.5.C Add keyboard/panel routing regression coverage.
- [ ] TD.5.D Add chunk merge and stale request tests.

## P3 - Context Compiler Lite (After PERF Track)

Goal: build bounded and controllable context assembly without recursive filesystem prompt dumping.

### P3.A - ContextBundle v0

- [ ] Define `ContextBundle` with `user_intent`, `workspace`, `recent_actions`, `selected_files`, `search_results`, and `token_budget`.
- [ ] Build context from managers only (Search/Workspace/History), no recursive filesystem scan.
- [ ] Apply token budget using initial char-count approximation.

### P3.B - Agent Integration

- [ ] Build `ContextBundle` before agent run.
- [ ] Do not inject full filesystem results into prompt.
- [ ] Use bounded preview for large content.
- [ ] Add prompt audit record for context composition.

## FEAT.11 - Learning Material Review (Blocked)

Blocked by: ADR-028, TD.5.A, PERF.1, TD.1.A, TD.2.A, TD.3.A.

- [ ] FEAT.11.A Add ADR and security model for local review boundaries.
- [ ] FEAT.11.B Add config schema under `[agent.local_context]` with disabled-by-default policy.
- [ ] FEAT.11.C Add typed DTOs for request/candidate/report/scan stats.
- [ ] FEAT.11.D Build metadata-first review manager based on existing search cache or selected roots.
- [ ] FEAT.11.E Add rule-based classifier for project/note/certificate/presentation/report classes.
- [ ] FEAT.11.F Add safe preview pipeline with byte cap, denylist, and redaction.
- [ ] FEAT.11.G Register approval-gated `learning_material_review` agent tool.
- [ ] FEAT.11.H Add report UI with scanned roots, candidate count, filtered count, and usage hints.
- [ ] FEAT.11.I Add approval-gated note draft and markdown export actions.
- [ ] FEAT.11.J Add regression tests for denied root, symlink escape, secret filter, prompt budget, and grounded output.
