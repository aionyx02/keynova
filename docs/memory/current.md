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

## Next Step

1. Proceed to PERF.2 Search Execution Bound (SearchService coordinator, bounded worker pool, cancellation token propagation, stale-request discard, backpressure, regression tests).
2. Or proceed to TD.4 (agent approval/cancel channelization, SearchService/Terminal actor alignment).
3. Keep FEAT.11 at planning/ADR level only until blockers clear.

## Known Risks

- If lifecycle refactor lands without measurement scripts, memory goals cannot be validated.
- If preview/read boundaries are mixed, private file exposure risk increases.
- If docs are not synchronized during refactor, task-state drift will reappear.
