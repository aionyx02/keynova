---
type: working_memory
status: active
priority: p0
updated: 2026-05-15
context_policy: always_retrievable
owner: project
---

# Current Project Memory

## Current Strategy

- Feature-first with safety-first runtime lifecycle.
- Retrieval-first docs workflow.
- Markdown remains source of truth without full-doc prompt dumping.

## Current Focus

- P0 Agent Completion Baseline COMPLETE (2026-05-13).
- P1 Workflow MVP COMPLETE (2026-05-13).
- P2 Dev Workflow Pack COMPLETE (2026-05-14).
- Mainline shifted to runtime stability track:
  - PERF.1 Low Memory Background Mode
  - PERF.2 Search Execution Bound
  - PERF.3 Heavy Feature Lazy Runtime
- P3 Context Compiler Lite remains planned after PERF track.
- FEAT.11 Learning Material Review remains blocked.

## Important Constraints

- LLM cannot directly operate unrestricted system commands.
- Approval-gated execution remains mandatory.
- Generic shell tool remains blocked until platform sandbox hardening is complete.
- "Background Core < 100 MB" excludes active WebView, loaded local LLM model memory, PTY terminal sessions, monitoring streams, and index rebuild tasks.

## Last Confirmed Progress

- P2.A: `dispatch_git_status` hardened with workspace scope, timeout, output bound, and preview field.
- P2.B: 4 approval-gated dev tools added: `dev.cargo_test`, `dev.cargo_check`, `dev.npm_build`, `dev.npm_lint`.
- P2.C: `dev.explain_compiler_error` tool added with bounded structured extraction.
- 196 tests passing with 1 pre-existing unrelated failure (`note_lazyvim_missing_nvim_returns_inline_guidance`).
- 2026-05-14: task planning updated from feasibility review (`Keynova_Agent_Architecture_Tasks_Review.docx`) and normalized into `docs/tasks/{active,backlog,blocked}.md`.
- 2026-05-15 TD.5.A gate confirmed (196/1). PERF.1.A–H complete:
  - `performance.low_memory_mode` setting added to `default_config.toml` + `settings_schema.rs`.
  - Bootstrap gates `start_prewarm` and `start_file_index` on the setting.
  - `ai.check_setup` TTL cache (300 s, keyed by provider/model/URL) added to `AiHandler`.
  - `ai.ollama_keep_alive` setting added; threaded through `chat_async` → `chat_ollama`.
  - PERF.1.G (model catalog) and PERF.1.H (LRU caps) verified already compliant.

- 2026-05-15 TD.1.A–C complete:
  - IPC centralized: `CommandPalette` + `TerminalPanel` use `useIPC().dispatch()` for all `cmd_dispatch` routes.
  - Search domain logic extracted to `src/utils/search.ts`.
  - Two lifecycle hooks extracted: `useWindowResize`, `useSearchMetadata`.
- PERF.1.I checklist added to `docs/testing.md` section 6.1.

- 2026-05-15 TD.3.A–C complete:
  - `src-tauri/src/models/ipc_requests.rs`: typed Deserialize DTOs for all critical routes (search, setting, terminal).
  - `handlers/setting.rs`, `handlers/terminal.rs`, `handlers/search.rs`: replaced raw `Value` field access with `serde_json::from_value::<DTO>`.
  - `src/ipc/routes.ts`: centralized `IPC.*` route constants (35 routes).
  - `src/ipc/types.ts`: typed request/response interfaces matching Rust DTOs.
  - `useCommands.ts` migrated from module-level `invoke` to `useIPC()` + constants; `dispatch` now in deps (stable via `IPCProvider`).
  - `useSearchMetadata.ts`, `useHotkey.ts`, `CommandPalette.tsx`, `TerminalPanel.tsx`: all inline route strings replaced with `IPC.*` constants.
  - `scripts/docs-sync.mjs` + `docs-guard.mjs`: CRLF normalization fix prevents duplicate frontmatter on Windows.

- 2026-05-15 TD.2.A–C complete:
  - `AppContainer` composition root created; `IPCProvider` provides stable `dispatch` reference; `App.tsx` renders `AppContainer`.
  - `FeatureContext` added: `FeatureProvider` + `useFeature()` with `activate(key: FeatureKey)` — boundary for PERF.3 lazy service activation.
  - `AppState::new()` refactored: `ManagerBundle`, `create_managers()`, `build_builtin_registry()`, `build_command_router()` extracted; `new()` reduced to ~15 lines.
  - `scripts/docs-sync.mjs` and `docs-guard.mjs` fixed: CRLF normalization added to prevent duplicate frontmatter injection on Windows.

- 2026-05-15 PERF.2.A–F complete:
  - `src-tauri/src/managers/search_service.rs`: `SearchService` with Condvar-based slot (at most one pending task), cancel token propagation via `Arc<AtomicBool>`.
  - `execute_stream_query` replaced `std::thread::spawn` per query with `search_service.submit()`.
  - `file_results_with_timeout` replaced by `file_results_bounded` with cancel token checks before and after platform search.
  - `StreamWorkerRequest` carries `cancel: Arc<AtomicBool>`; `run_stream_worker` uses it for fast early exit (no mutex needed).
  - 4 deterministic regression tests in `managers::search_service::tests` — all passing.
  - 200/201 tests pass (1 pre-existing unrelated failure unchanged).

- 2026-05-15 PERF.3.A–E complete:
  - PERF.3.A: `start_prewarm` removed from bootstrap; terminal cold-starts on demand.
  - PERF.3.B+C: Verified already done (system monitor stop-on-close; nvim lazy detect).
  - PERF.3.D: `AiHandler.in_flight` rejects concurrent chat; `ai.unload` sends keep_alive=0 to Ollama; `chat_async` gains `completion_flag` parameter.
  - PERF.3.E: `FeatureContext.activate()` → `feature.activate` IPC → `FeatureHandler` → `start_prewarm`; `TerminalPanel` calls `activate("terminal")` on mount.
  - Clippy clean; 200/201 tests pass (1 pre-existing failure).

- 2026-05-15 TD.4.A–C complete:
  - `AgentRuntime`: `run_notify: (Mutex<()>, Condvar)` replaces 100ms poll in `wait_for_react_approval`.
  - `SearchService`: `Slot::Shutdown` + `shutdown()` complete actor lifecycle.
  - `TerminalManager` actor boundary verified (no code changes required).

- 2026-05-15 TD.5.B–D complete:
  - `tauri.conf.json`: CSP hardened with `object-src 'none'`, `frame-src 'none'`, `base-uri 'self'`, `worker-src 'none'`.
  - `web.rs`: `validate_searxng_url` blocks HTTP to non-localhost SearXNG endpoints.
  - `settings_schema.rs`: `security.network_allowlist` setting added.
  - `command_router.rs`: 6 routing regression tests (TD.5.C).
  - `handlers/search.rs`: 5 chunk merge + stale request tests (TD.5.D).
  - 216/1 tests passing (pre-existing failure unchanged).

- 2026-05-15 AI panel UX fix complete:
  - Added `agent.clear_runs` so AGENT run history and memory refs can be reset.
  - `AiPanel` clear button now clears by active mode (`CHAT` or `AGENT`).
  - `useAgent` run ordering changed to chronological (old -> new) for stable latest-at-bottom flow.
  - `useAi` now guards against stale `ai.get_history` hydration overriding post-clear/post-send state.
- 2026-05-15 Setting panel tab density fix complete:
  - `SettingPanel` tabs now render at intrinsic width with horizontal scroll instead of compressed equal-width layout.
  - dynamic section label fallback capitalization added to improve readability for non-mapped sections (for example `performance`, `security`).
- 2026-05-15 Setting panel visual polish complete:
  - Added dark-theme scrollbar skin for section rail (`.setting-tabs-scroll`) in `src/index.css`.
  - Added left/right gradient edge masks on the tab rail to blend with existing panel backdrop.
  - Adjusted tab text/spacing to better match command palette visual density.

## Next Step

1. Proceed to P3 Context Compiler Lite (define `ContextBundle`, build from managers, apply token budget).
2. Keep FEAT.11 at planning/ADR level only until blockers clear.

## Known Risks

- If lifecycle refactor lands without measurement scripts, memory goals cannot be validated.
- If preview/read boundaries are mixed, private file exposure risk increases.
- If docs are not synchronized during refactor, task-state drift will reappear.
