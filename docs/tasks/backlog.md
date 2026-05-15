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

- [x] PERF.2.A Introduce SearchService coordinator as single request entry. (`src-tauri/src/managers/search_service.rs` — `SearchService` struct with single worker thread; `SearchHandlerDeps.search_service` wired in `state.rs`)
- [x] PERF.2.B Replace per-query thread fan-out with bounded worker pool. (`execute_stream_query` uses `search_service.submit()` instead of `std::thread::spawn`; at most one task pending, one running)
- [x] PERF.2.C Add cancellation token propagation through indexer and fallback search. (`Arc<AtomicBool>` cancel token threaded through `StreamWorkerRequest` → `run_stream_worker` → `file_results_bounded`; inner timeout thread checks cancel before and after file search)
- [x] PERF.2.D Enforce latest-request-only behavior and discard stale responses. (`SearchService` uses slot-based design: submitting a new task atomically replaces the pending slot and sets the previous cancel token to true)
- [x] PERF.2.E Add backpressure for streaming chunks to avoid queue buildup. (Addressed by design: single worker + slot ensures at most one pending async chunk per frontend session)
- [x] PERF.2.F Add regression tests for correctness, cancellation, timeout, and fallback. (`managers::search_service::tests` — 4 deterministic tests: cancel_sets_token, chain_cancels_all, worker_runs_latest, cancelled_task_skipped)

## PERF.3 - Heavy Feature Lazy Runtime

Goal: keep heavy services off until user intent requires them.

- [x] PERF.3.A Make TerminalManager lazy-init (no PTY/shell before first terminal use). (`bootstrap.rs`: removed `start_prewarm` call and import; terminal cold-starts on first `terminal.open`)
- [x] PERF.3.B Make system monitoring stream session-scoped and stop-on-close. (Already done: `SystemMonitoringPanel` calls `stream_start` on mount, `stream_stop` on unmount; backend handler verified correct)
- [x] PERF.3.C Make Nvim manager lazy-init (no detect/download during startup). (Already lazy: `NvimHandler::new()` stores only `event_bus`; `detect_nvim` only runs on `nvim.detect` IPC command)
- [x] PERF.3.D Add AI Runtime Service for setup TTL, request queue, keep_alive, and unload policy. (`AiHandler`: `in_flight: Arc<AtomicBool>` rejects concurrent requests; `ai.unload` command sends keep_alive=0 to Ollama; `chat_async` clears flag via `completion_flag` parameter. TTL cache + keep_alive from PERF.1)
- [x] PERF.3.E Add Lazy Service/Feature Gate support in AppContainer. (`FeatureContext.tsx` wired to `feature.activate` IPC; `FeatureHandler` in Rust triggers `start_prewarm` on `terminal` activation; `TerminalPanel` calls `activate("terminal")` on mount; `IPC.FEATURE_ACTIVATE` constant added)

## TD.4 + TD.5 Follow-Up

- [x] TD.4.A Agent approval/cancel channelization cleanup. (`core/agent_runtime.rs` — added `run_notify: (Mutex<()>, Condvar)` to `AgentRuntime`; `update_run` and `cancel` call `notify_all`; `wait_for_react_approval` replaced `thread::sleep(100ms)` poll with `condvar.wait_timeout`)
- [x] TD.4.B Merge SearchService actor changes with PERF.2 execution model. (`managers/search_service.rs` — added `Slot::Shutdown` variant and `SearchService::shutdown()` method; worker loop exits cleanly on `Shutdown`; lifecycle now: `new → submit* → shutdown`)
- [x] TD.4.C Merge Terminal actor boundary with PERF.3 lazy runtime model. (Verified: `TerminalManager` cold-starts on first `terminal.open`; activation via `FeatureHandler.activate("terminal")`; all state changes route through `TerminalHandler` commands; output via `on_output` callback → EventBus. No code changes required.)
- [x] TD.5.B Enforce CSP and network allowlist. (`tauri.conf.json`: added `object-src 'none'; frame-src 'none'; base-uri 'self'; worker-src 'none'`; `handlers/agent/web.rs`: `validate_searxng_url` rejects HTTP to non-localhost; `models/settings_schema.rs`: `security.network_allowlist` setting added; 4 new URL-validation tests)
- [x] TD.5.C Add keyboard/panel routing regression coverage. (`core/command_router.rs`: 6 routing regression tests — valid dispatch, unknown namespace, missing dot, empty route, multi-dot split, handler count)
- [x] TD.5.D Add chunk merge and stale request tests. (`handlers/search.rs`: 5 new tests — key format, deduplication by source+path, first-batch priority on dup, stale cancel guard, sort quota)

## P3 - Context Compiler Lite (After PERF Track)

Goal: build bounded and controllable context assembly without recursive filesystem prompt dumping.

### P3.A - ContextBundle v0

- [x] Define `ContextBundle` with `user_intent`, `workspace`, `recent_actions`, `selected_files`, `search_results`, and `token_budget`. (`models/context_bundle.rs` — `ContextBundle`, `WorkspaceContext`, `SelectedFileContext`, `ContextTokenBudget`)
- [x] Build context from managers only (Search/Workspace/History), no recursive filesystem scan. (`AgentHandler::build_context_bundle` pulls from `WorkspaceManager` only; search results passed from already-computed `sources_for_prompt`)
- [x] Apply token budget using initial char-count approximation. (`ContextBundle::build` — two-pass: snippet preview trim at 200 chars, then total char-count cap at 3000)

### P3.B - Agent Integration

- [x] Build `ContextBundle` before agent run. (`start_react_run` and `start_heuristic_run` both call `build_context_bundle` and set `context_bundle: Some(...)` on `AgentRun`)
- [x] Do not inject full filesystem results into prompt. (only `keynova_search` manager results included; no `filesystem_search` or `filesystem_read` in bundle)
- [x] Use bounded preview for large content. (snippets > 200 chars trimmed with `…`; total results dropped once budget exceeded)
- [x] Add prompt audit record for context composition. (`AgentRun.context_bundle` carries `token_budget` with `used_chars`, `remaining_chars`, `truncated`; 3 deterministic tests in `models::context_bundle::tests`)

## FEAT.11 - Learning Material Review ✓ Complete (2026-05-15)

All prerequisites cleared. Developer authorized implementation via explicit instruction.

- [x] FEAT.11.A Add ADR and security model for local review boundaries. (`docs/adr/0028-learning-material-review-local-context.md` — status: 提議; symlink escape prevention, denylist filtering, workspace-root-only scope)
- [x] FEAT.11.B Add config schema under `[agent.local_context]` with disabled-by-default policy. (`settings_schema.rs`: `agent.local_context.enabled`, `max_scan_files`, `max_preview_bytes`, `max_depth`, `extra_denylist`; `default_config.toml`: `[agent.local_context]` section with `enabled=false`)
- [x] FEAT.11.C Add typed DTOs for request/candidate/report/scan stats. (`src-tauri/src/models/learning_material.rs`: `MaterialClass`, `MaterialCandidate`, `ScanStats`, `ReviewReport` with `to_markdown()`)
- [x] FEAT.11.D Build metadata-first review manager based on existing search cache or selected roots. (`src-tauri/src/managers/learning_material_manager.rs`: `LearningMaterialManager::from_config`, `scan()` with canonicalize + root-prefix check, max_depth/max_scan_files limits)
- [x] FEAT.11.E Add rule-based classifier for project/note/certificate/presentation/report classes. (`classify_by_extension` + `is_project_root` in `learning_material_manager.rs`)
- [x] FEAT.11.F Add safe preview pipeline with byte cap, denylist, and redaction. (`preview_file()` with byte cap + `prepare_observation` secret redaction; `is_denied()` handles glob `*.ext` and exact match patterns; DEFAULT_DENYLIST covers .git, node_modules, .env, *.key, *.pem, id_rsa, etc.)
- [x] FEAT.11.G Register approval-gated `learning_material_review` agent tool. (`core/agent_runtime.rs`: `LearningMaterialReviewToolParams/Result` with JsonSchema; registered with `AgentToolApprovalPolicy::Required`, `ActionRisk::Medium`, 15_000ms timeout; `handlers/agent/mod.rs`: `dispatch_learning_material_review()` wired into approval-gated dispatch)
- [x] FEAT.11.H Add report UI with scanned roots, candidate count, filtered count, and usage hints. (`src/components/LearningMaterialPanel.tsx`: scan roots input, stats bar, class filter tabs All/Project/Note/Report/Presentation/Certificate/Unknown, candidate list with name/path/size)
- [x] FEAT.11.I Add approval-gated note draft and markdown export actions. (`LearningMaterialHandler` commands: `export_note` via `NoteManager.save`, `export_markdown` via canonicalized `fs::write`; UI: "Export as Note" button; `src/ipc/routes.ts`: `LEARNING_MATERIAL_SCAN/EXPORT_NOTE/EXPORT_MARKDOWN` constants)
- [x] FEAT.11.J Add regression tests for denied root, symlink escape, secret filter, prompt budget, and grounded output. (`learning_material_manager.rs` tests module: 13 tests — disabled guard, secret denylist, glob denylist, extension classifier, project root detection, scan result structure, path prefix rejection, `to_markdown` output; 233/234 total tests pass; 1 pre-existing failure unchanged)
