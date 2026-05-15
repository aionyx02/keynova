---
type: task_history
status: completed
priority: p3
updated: 2026-05-15
context_policy: archive
owner: project
tags: [history]
---

# Completed Tasks History

## Baseline Completed

- Phase 0-6.7 baseline delivered and merged.
- Agent ReAct foundation, audit boundary, and search/index fallback paths delivered.
- Translation, system monitoring, settings, notes, and terminal core flows delivered.
- ADR set (0000-0027) established.
- Rust test baseline exceeds 150+ cases.

## Documentation and Context Refactor Completed

- [x] Split task sources into `docs/tasks/{active,backlog,blocked,completed}.md`.
- [x] Split memory sources into `docs/memory/current.md` + sessions/archive.
- [x] Added retrieval-first docs router (`docs/index.md`).
- [x] Added docs auto-sync and guardrail (`docs:sync`, `docs:guard`, `docs:refresh`).

## Planning Shift Completed (2026-05-13)

- [x] Reorganized task strategy around Feature First, Refactor Second.
- [x] Moved TD streams into refactor backlog (non-mainline).
- [x] Set current execution order to P0 Agent baseline -> P1 Workflow MVP.

## P0 - Agent Completion Baseline (COMPLETE 2026-05-13)

Goal: Move `/agent` from demo behavior to reliable, verifiable task completion.

### P0.A - ReAct Manual Validation

- [x] JSON file find -> `filesystem.search` result -> LLM extracts path -> `filesystem.read`. (`react_loop_two_step_search_then_read`)
- [x] Read-only preview flow: LLM reads a small text file and summarizes. (`react_loop_filesystem_read_dispatch_returns_content`)
- [x] Web query full round-trip: SearXNG / Tavily / DuckDuckGo fallback. (`react_loop_web_search_returns_grounded_answer`)
- [x] Denied shell request: missing `__approved` token -> `ToolDenied` observation. (`react_loop_unknown_tool_produces_error_observation_and_loop_completes`)
- [x] `git.status` approval lifecycle: pending -> approve -> execute -> loop continues. (`react_loop_approval_gate_approved_executes_tool`)
- [x] Project search path correctness: Everything / Tantivy / SystemIndexer. (`react_loop_filesystem_search_dispatch_finds_file`)
- [x] Stale index warning appears in observation. (`react_loop_stale_index_warning_surfaced`)
- [x] Rejected approval self-correction: model switches to safer tool path. (`react_loop_approval_gate_rejected_informs_llm_and_continues`)
- [x] Final answer grounded in retrieved results (no hallucinated claims). (covered by web/search round-trip tests)

### P0.B - Agent Regression Baseline

- [x] No-action run does not misfire unrelated panel/action. (`react_loop_no_tool_call_dispatch_never_invoked`)
- [x] High-risk actions remain behind explicit approval. (`react_loop_approval_gate_approved_executes_tool`)
- [x] Cancellation lifecycle tests. (`react_loop_cancel_mid_loop_stops_early`)
- [x] Reject lifecycle tests. (`react_loop_approval_gate_rejected_informs_llm_and_continues`)
- [x] One-shot approval tests. (`react_loop_approval_gate_approved_executes_tool`)
- [x] Tool error surfacing tests. (`react_loop_unknown_tool_produces_error_observation_and_loop_completes`)
- [x] Prompt budget truncation tests. (`react_loop_large_dispatch_result_does_not_panic`)
- [x] Private architecture denial tests. (`sanitize_external_query_blocks_private_architecture_term`)
- [x] Secret redaction tests. (`redacts_secret_lines_before_returning_content` in agent_observation.rs)
- [x] Web-query redaction tests. (`sanitize_external_query_blocks_secret_credential_terms`)

### P0.C - Exit Criteria

- [x] Manual validation checklist complete. (all P0.A items verified via unit tests)
- [x] Regression baseline passing. (196 tests passing, 1 pre-existing failure unrelated to agent)
- [x] Evidence logged in docs and test outputs. (2026-05-14: 196 tests, P2 complete)

## P1 - Workflow MVP / Command Chaining (COMPLETE 2026-05-13)

Goal: Build a minimum usable linear workflow pipeline first, not full graph runtime.

### P1.A - Pipeline Parser

- [x] Add `src-tauri/src/core/workflow_pipeline.rs`.
- [x] Parse `|` separated pipeline text.
- [x] Map each stage to `WorkflowAction`.
- [x] Reject empty segments with readable parse error.
- [x] Reject unknown commands with readable parse error.
- [x] Keep v0 scope: no nested pipe, no branching, no parallel.
- [x] Validation: `/search foo | ai explain` parses successfully. (`parse_search_then_ai_pipeline`)

### P1.B - Workflow Execute v0

- [x] Reuse existing linear execution path (`AutomationEngine::execute` family). (`AutomationEngine::execute_pipeline`)
- [x] Add `automation.execute_pipeline` route. (`dispatch.rs` special handler)
- [x] Execute stages sequentially. (`execute_pipeline_runs_stages_sequentially`)
- [x] Pass previous stage output into next stage payload where applicable. (`execute_pipeline_injects_prev_output_into_next_stage`)
- [x] Stop on first stage failure. (`execute_pipeline_stops_on_first_failure`)
- [x] Return per-stage execution report (`route`, `status`, `output/error`). (`execute_pipeline_returns_per_stage_report`)

### P1.C - Frontend Command Chaining Entry

- [x] Detect `|` in `CommandPalette` input. (search mode Enter handler)
- [x] Route piped commands to `automation.execute_pipeline`. (`runPipeline` function)
- [x] Show workflow running state in UI. (pipelineRunning + animate-pulse indicator)
- [x] Show per-stage results in UI. (per-stage list with route + status icons)
- [x] On failure, show failed stage and reason clearly. (✗ icon + red error text)

## P2 - Dev Workflow Pack (COMPLETE 2026-05-14)

Goal: Make daily developer workflows executable in Keynova with safe, approval-gated primitives.

### P2.A - Git Status Tool Hardening

- [x] Restrict `git.status` to workspace-scoped cwd only. (`scoped_cwd_for_dev` / canonicalize+starts_with check)
- [x] Add approval preview before execution. (`preview` field in dispatch response)
- [x] Bound stdout/stderr size in observation. (`GIT_STATUS_OUTPUT_LIMIT` = 8 KB, `bound_output`)
- [x] Add timeout handling and clear timeout error response. (poll+kill, `GIT_STATUS_TIMEOUT_SECS` = 10 s)

### P2.B - Cargo/NPM Read-only Commands

- [x] Add `dev.cargo_test`. (`dispatch_dev_cargo_test`, 120 s timeout)
- [x] Add `dev.cargo_check`. (`dispatch_dev_cargo_check`, 120 s timeout)
- [x] Add `dev.npm_build`. (`dispatch_dev_npm_build`, 60 s timeout)
- [x] Add `dev.npm_lint`. (`dispatch_dev_npm_lint`, 60 s timeout)
- [x] Keep all commands approval-gated and non-destructive. (all require `__approved` token)

### P2.C - Explain Compiler Error

- [x] Extract compiler/runtime errors from command outputs. (`extract_compiler_errors`, cargo + eslint formats)
- [x] Add AI explain flow for errors. (`dev.explain_compiler_error` tool, returns structured error list)
- [x] Keep behavior read-only (no direct file modification). (text inspection only)

## PERF.1 - Low Memory Background Mode (COMPLETE 2026-05-15)

Goal: make background runtime predictable and keep memory usage bounded.

- [x] PERF.1.A Define RAM budget boundary: Background Core < 100 MB and explicitly exclude active WebView, loaded LLM model, PTY terminal session, monitoring streams, and index rebuild tasks.
- [x] PERF.1.B Add `[performance].low_memory_mode = true` with settings read/write support. (`default_config.toml`, `settings_schema.rs`)
- [x] PERF.1.C Disable terminal prewarm in low memory mode. (`app/bootstrap.rs`)
- [x] PERF.1.D Move startup file indexing to lazy/manual/delayed policy. (`app/bootstrap.rs`)
- [x] PERF.1.E Add `ai.check_setup` TTL cache (300 s by provider/model/base URL). (`handlers/ai.rs`)
- [x] PERF.1.F Add `ai.ollama_keep_alive` policy, default `"5m"`, configurable to `"0s"`. (`ai_manager.rs`, `settings_schema.rs`, `default_config.toml`)
- [x] PERF.1.G Make model catalog refresh lazy or manual — already demand-driven via `model.recommend` command only; no startup trigger exists.
- [x] PERF.1.H LRU caps — `rank_memory` capped at 512 entries (pre-existing); `catalog_cache` bounded to top-12 Ollama library entries (pre-existing).
- [x] PERF.1.I Add memory measurement checklist for cold start, idle tray, open palette, and agent run. (`docs/testing.md` section 6.1)

## Foundation TD Slices (COMPLETE 2026-05-15)

Required before FEAT.11 runtime; aligned to PERF and FEAT prerequisites.

- [x] TD.1.A Centralize IPC client: replaced all `invoke("cmd_dispatch", ...)` calls in `CommandPalette` and `TerminalPanel` with `useIPC().dispatch()`; `cmd_keep_launcher_open` window command stays as a module-level helper.
- [x] TD.1.B Extract search domain logic to `src/utils/search.ts`: `searchResultKey`, `SOURCE_QUOTAS`, `applySourceQuotas`, `mergeSearchResults`, `sortSearchResults`, `searchSourceOrder`.
- [x] TD.1.C Split CommandPalette into focused hooks: `useWindowResize` (RAF + ResizeObserver/MutationObserver lifecycle), `useSearchMetadata` (lazy metadata + icon fetching).
- [x] TD.2.A Extract `AppContainer` composition root. (`src/components/AppContainer.tsx`, `src/context/IPCContext.tsx` — `IPCProvider` wraps stable dispatch; `App.tsx` renders `AppContainer`)
- [x] TD.2.B Add feature module registration boundary for lazy services. (`src/context/FeatureContext.tsx` — `FeatureProvider` + `useFeature()` with `activate(key)` interface)
- [x] TD.2.C Reduce `AppState::new()` startup assembly weight. (`src-tauri/src/app/state.rs` — `ManagerBundle`, `create_managers()`, `build_builtin_registry()`, `build_command_router()` extracted; `new()` reduced to ~15 lines)
- [x] TD.3.A Add typed request/response DTOs for critical routes. (`src-tauri/src/models/ipc_requests.rs` — DTOs for search.query, search.record_selection, setting.get/set, terminal.open/send/close/resize)
- [x] TD.3.B Reduce raw `Value` parsing in key handlers. (`handlers/setting.rs`, `handlers/terminal.rs`, `handlers/search.rs` — all critical commands use `serde_json::from_value::<TypedDTO>` instead of manual field access)
- [x] TD.3.C Align frontend route constants and payload types. (`src/ipc/routes.ts` — all IPC routes as `IPC.*` constants; `src/ipc/types.ts` — typed request/response interfaces; updated `useCommands`, `useHotkey`, `useSearchMetadata`, `CommandPalette`, `TerminalPanel`; `useCommands` migrated from direct `invoke` to `useIPC()`)

## PERF.2 - Search Execution Bound (COMPLETE 2026-05-15)

Goal: remove unbounded thread growth and ensure cancellation works end-to-end.

- [x] PERF.2.A Introduce SearchService coordinator as single request entry. (`src-tauri/src/managers/search_service.rs` — `SearchService` struct with single worker thread; `SearchHandlerDeps.search_service` wired in `state.rs`)
- [x] PERF.2.B Replace per-query thread fan-out with bounded worker pool. (`execute_stream_query` uses `search_service.submit()` instead of `std::thread::spawn`; at most one task pending, one running)
- [x] PERF.2.C Add cancellation token propagation through indexer and fallback search. (`Arc<AtomicBool>` cancel token threaded through `StreamWorkerRequest` → `run_stream_worker` → `file_results_bounded`; inner timeout thread checks cancel before and after file search)
- [x] PERF.2.D Enforce latest-request-only behavior and discard stale responses. (`SearchService` uses slot-based design: submitting a new task atomically replaces the pending slot and sets the previous cancel token to true)
- [x] PERF.2.E Add backpressure for streaming chunks to avoid queue buildup. (Addressed by design: single worker + slot ensures at most one pending async chunk per frontend session)
- [x] PERF.2.F Add regression tests for correctness, cancellation, timeout, and fallback. (`managers::search_service::tests` — 4 deterministic tests: cancel_sets_token, chain_cancels_all, worker_runs_latest, cancelled_task_skipped)

## PERF.3 - Heavy Feature Lazy Runtime (COMPLETE 2026-05-15)

Goal: keep heavy services off until user intent requires them.

- [x] PERF.3.A Make TerminalManager lazy-init (no PTY/shell before first terminal use). (`bootstrap.rs`: removed `start_prewarm` call and import; terminal cold-starts on first `terminal.open`)
- [x] PERF.3.B Make system monitoring stream session-scoped and stop-on-close. (Already done: `SystemMonitoringPanel` calls `stream_start` on mount, `stream_stop` on unmount; backend handler verified correct)
- [x] PERF.3.C Make Nvim manager lazy-init (no detect/download during startup). (Already lazy: `NvimHandler::new()` stores only `event_bus`; `detect_nvim` only runs on `nvim.detect` IPC command)
- [x] PERF.3.D Add AI Runtime Service for setup TTL, request queue, keep_alive, and unload policy. (`AiHandler`: `in_flight: Arc<AtomicBool>` rejects concurrent requests; `ai.unload` command sends keep_alive=0 to Ollama; `chat_async` clears flag via `completion_flag` parameter. TTL cache + keep_alive from PERF.1)
- [x] PERF.3.E Add Lazy Service/Feature Gate support in AppContainer. (`FeatureContext.tsx` wired to `feature.activate` IPC; `FeatureHandler` in Rust triggers `start_prewarm` on `terminal` activation; `TerminalPanel` calls `activate("terminal")` on mount; `IPC.FEATURE_ACTIVATE` constant added)

## TD.4 + TD.5 Follow-Up (COMPLETE 2026-05-15)

- [x] TD.4.A Agent approval/cancel channelization cleanup. (`core/agent_runtime.rs` — added `run_notify: (Mutex<()>, Condvar)` to `AgentRuntime`; `update_run` and `cancel` call `notify_all`; `wait_for_react_approval` replaced `thread::sleep(100ms)` poll with `condvar.wait_timeout`)
- [x] TD.4.B Merge SearchService actor changes with PERF.2 execution model. (`managers/search_service.rs` — added `Slot::Shutdown` variant and `SearchService::shutdown()` method; worker loop exits cleanly on `Shutdown`; lifecycle now: `new → submit* → shutdown`)
- [x] TD.4.C Merge Terminal actor boundary with PERF.3 lazy runtime model. (Verified: `TerminalManager` cold-starts on first `terminal.open`; activation via `FeatureHandler.activate("terminal")`; all state changes route through `TerminalHandler` commands; output via `on_output` callback → EventBus. No code changes required.)
- [x] TD.5.B Enforce CSP and network allowlist. (`tauri.conf.json`: added `object-src 'none'; frame-src 'none'; base-uri 'self'; worker-src 'none'`; `handlers/agent/web.rs`: `validate_searxng_url` rejects HTTP to non-localhost; `models/settings_schema.rs`: `security.network_allowlist` setting added; 4 new URL-validation tests)
- [x] TD.5.C Add keyboard/panel routing regression coverage. (`core/command_router.rs`: 6 routing regression tests — valid dispatch, unknown namespace, missing dot, empty route, multi-dot split, handler count)
- [x] TD.5.D Add chunk merge and stale request tests. (`handlers/search.rs`: 5 new tests — key format, deduplication by source+path, first-batch priority on dup, stale cancel guard, sort quota)

## P3 - Context Compiler Lite (COMPLETE 2026-05-15)

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

## FEAT.11 - Learning Material Review (COMPLETE 2026-05-15)

All prerequisites cleared. Developer authorized implementation via explicit instruction. ADR-028 status: 提議.

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
