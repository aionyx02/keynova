---
type: working_memory
status: active
priority: p0
updated: 2026-05-18
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
- PERF.1/2/3 + TD.1/2/3/4/5 + P3 + FEAT.11 + Phase 7a + LAUNCH.1 全段 COMPLETE (2026-05-18)。
- 下一個 phase 待選擇（無 ADR 阻擋入口：UTIL.1/2、ONBOARD.1、LAUNCH.2、NOTE.1）。

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

- 2026-05-15 P3.A–B complete:
  - `models/context_bundle.rs`: `ContextBundle` with `user_intent`, `workspace` (`WorkspaceContext`), `recent_actions`, `selected_files` (`SelectedFileContext`), `search_results`, and `token_budget` (`ContextTokenBudget`).
  - `ContextBundle::build()`: two-pass budget — snippets > 200 chars trimmed, total char-count capped at 3000; no recursive filesystem scan.
  - `AgentHandler::build_context_bundle()`: pulls workspace metadata + recent actions + recent file paths from `WorkspaceManager`; reuses pre-computed `keynova_search` results.
  - `AgentRun.context_bundle: Option<ContextBundle>` added (`#[serde(default)]` for backward compat).
  - Both `start_react_run` and `start_heuristic_run` set `context_bundle: Some(...)`.
  - Frontend: `ContextBundle`, `WorkspaceContext`, `SelectedFileContext`, `ContextTokenBudget` types added to `useAgent.ts`; `AgentRun.context_bundle` field added.
  - 3 new deterministic tests in `models::context_bundle::tests`; 220/221 total (1 pre-existing failure unchanged).

- 2026-05-15 FEAT.11 Learning Material Review complete (FEAT.11.A–J):
  - ADR-028 created (`docs/adr/0028-learning-material-review-local-context.md`), status 提議; developer authorized implementation via explicit instruction.
  - `models/learning_material.rs`: `MaterialClass` enum, `MaterialCandidate`, `ScanStats`, `ReviewReport` with `to_markdown()`.
  - `managers/learning_material_manager.rs`: `LearningMaterialManager::from_config`; `scan()` with canonicalize + root-prefix enforcement + symlink escape prevention; `classify_by_extension`; `is_project_root`; `is_denied` (glob `*.ext` + exact); `preview_file` with byte cap + `prepare_observation` redaction; 13 tests.
  - `handlers/learning_material.rs`: `scan`, `preview`, `export_note`, `export_markdown` IPC commands; `export_markdown` canonicalizes target path before write.
  - `core/agent_runtime.rs`: `learning_material.review` agent tool (ApprovalPolicy::Required, ActionRisk::Medium, 15_000ms); tool count updated to 6.
  - `handlers/agent/mod.rs`: `dispatch_learning_material_review()` in approval-gated dispatch.
  - 5 settings under `agent.local_context` (`enabled=false` default); `default_config.toml` `[agent.local_context]` section added.
  - `src/components/LearningMaterialPanel.tsx`: scan roots input, stats bar, class filter tabs, candidate list, export-as-note action.
  - `src/ipc/routes.ts`: `LEARNING_MATERIAL_SCAN/EXPORT_NOTE/EXPORT_MARKDOWN` constants.
  - 233/234 total tests pass (1 pre-existing failure unchanged); clippy clean.

- 2026-05-17 LAUNCH.1.A slice 1 scaffold in progress:
  - `src/components/SecondaryActionMenu.tsx`: keyboard-driven menu shell (focus index + Enter/Esc).
  - `src/utils/secondaryActions.ts`: `buildSecondaryActions()` + `basenameFromPath()` + `SecondaryActionId` typed registry.
  - `src/components/CommandPalette.tsx`: menu open/close state, metadata expand toggle, copy-hint surface; integrates `revealItemInDir` from `@tauri-apps/plugin-opener`.
  - `src/ipc/routes.ts`: reserved 8 `file.*` route constants for incremental backend wiring (`open_with` / `reveal` / `rename` / `move` / `delete` / `hash` / `preview` / `open_as_text`).
  - `src-tauri/src/handlers/file.rs` (new) + `handlers/mod.rs` + `app/state.rs`: `FileHandler` skeleton registered in `build_command_router` (no behaviour wired yet — placeholder for slice 2-4).
  - Out-of-scope this commit: prompt engineering init template (`Prompt_Engineering_Init_Template.docx` + `scripts/generate_prompt_engineering_doc.py`) — cross-project reusable docs scaffold derived from current CLAUDE.md + docs/ pattern.

- 2026-05-18 LAUNCH.1.C/D/E 全段交付（LAUNCH.1 group 結案）：
  - 後端 `core/preview.rs` 新檔，抽出 `classify_path` / `read_text_preview` / `guess_image_mime`；`LearningMaterialManager::preview_file` 改呼叫之。
  - `handlers/file.rs` 加 `"preview"` arm，text 走 redact_secrets bounded read（4 KB / 500 lines default，64 KiB / 2000 lines cap），image 只回 metadata + mime，binary 只回 metadata。
  - `models/action.rs` `UiSearchItem` 加 `ScoreBreakdown { base, recency_boost, frequency_boost }` 欄位；`SearchManager::rank_boost_breakdown` 取代 `rank_boost`；`handlers/search.rs::apply_rank_boost` 寫入三段拆解。
  - `tauri.conf.json` 加 `app.security.assetProtocol = { enable: true, scope: ["**"] }`，`Cargo.toml` 加 `protocol-asset` feature。`docs/security.md` 新增 §10 邊界說明（read-only、與既有 file IPC 對齊、禁止用途）。
  - 前端新增 `src/components/{FilterChips,PreviewPane,RankTooltip}.tsx` 與 `src/hooks/useFilePreview.ts`；`useWindowResize.ts` 加 `widthRef` 參數，預設 640 px，preview 顯示時動態切到 960 px。
  - `CommandPalette` 改 grid 佈局（`grid-cols-[1fr_320px]`），`relative` anchor 從 outer 搬到 inner left wrapper（保留 SecondaryActionMenu 落位）。
  - 設定 `search.preview_enabled` / `search.show_rank_breakdown` 預設 true，配 `config-reloaded` event 監聽。
  - 測試：file::tests 22（+6）、core::preview::tests 7（新）、search_manager 4 個 breakdown 測試；總計 276/277（pre-existing nvim test 不變）。clippy / lint / tsc 全清。

- 2026-05-18 LAUNCH.1.A 收尾 + LAUNCH.1.B 全段交付：
  - 後端 `src-tauri/src/handlers/file.rs`: 5 個新 match arm — `rename` / `move` / `delete` / `hash` / `open_as_text`，全走 typed DTO (`serde_json::from_value`)。Destructive 三個採二段式 confirm gate（`confirm != true` 回 `{ preview: true, ... }`）。`delete` 用 `trash::delete()`（OS recycle bin）；`hash` 串流 64 KiB chunks 餵 `sha2::Sha256`，回 `{ algorithm, path, hex, bytes }`，只支援 sha256；`open_as_text` 走 `text_editor_for_platform()` (Windows `notepad.exe` / macOS `TextEdit` / Linux fallback to xdg-open)。
  - Typed DTO `src-tauri/src/models/ipc_requests.rs`: `FileRenameRequest` / `FileMoveRequest` / `FileDeleteRequest` / `FileHashRequest` / `FileOpenAsTextRequest`，全部 `#[serde(default)]` 給可選欄位 + `default_hash_algo()` helper。
  - Cargo deps `src-tauri/Cargo.toml`: `trash = "5"`、`sha2 = "0.10"`、`tempfile = "3"` (dev-dep)。trash 跨平台 (Windows SHFileOperationW / macOS NSFileManager trashItem / Linux freedesktop gio)，MIT/Apache-2.0，停在既有 file boundary 內不需 ADR。
  - 前端 `src/utils/secondaryActions.ts`: 解除 `open_with` disabled；依 `result.kind` 條件追加 `open_as_text` (file only) / `rename` / `move` / `delete` (non-app) / `hash` (file only)；新增 `isDestructive()` + `parentDirFromPath()` helper。
  - 前端 `src/components/SecondaryActionMenu.tsx`: 加 `pendingConfirmId` / `inlineInput` / `onInlineInputChange` / `onInlineInputKeyDown` props；focused destructive 行下方渲染紅框 confirm row；rename/move focus 時於該列下方渲染 inline `<input>`（autoFocus）；risk 顏色（high→red、medium→amber、low→gray）。
  - 前端 `src/components/CommandPalette.tsx`: 新 state `pendingConfirm` / `inlineInput` + helper `showHint(msg, durationMs)`；`handleSecondaryAction` 6 個 case 全填；Esc 全域與 selection 切換都清空 confirm 狀態。Toast 共用 `setCopyHint` + `copyResetRef`。
  - 測試 `handlers::file::tests`: 16 個 (11 新 + 5 既有)，含 SHA-256 known vector (`b"abc"` → `ba7816bf…`) 與 `delete_moves_to_trash_when_confirmed`（`#[ignore]`，需 desktop session）。全測 258/259 (pre-existing `note_lazyvim_missing_nvim_returns_inline_guidance` 不變)；`cargo clippy -- -D warnings` 清；`npx tsc --noEmit` 清；`npm run lint` 清。
  - LAUNCH.1.A + LAUNCH.1.B 搬至 `docs/tasks/completed.md`；LAUNCH.1.C/D/E 留 backlog。

## Next Step

LAUNCH.1.C/D/E 已於 2026-05-18 完成，LAUNCH.1 全段結案。

待使用者選擇下一個起手 phase；其餘無 ADR 阻擋入口：

- Phase 8a — UTIL.1 / UTIL.2：calculator++ 與 dev utilities。
- Phase 8c — ONBOARD.1：first-run tour、`?` cheatsheet。
- Phase 12 — LAUNCH.2：workspace-aware search 與 hotkey 切換。
- Phase 12 — NOTE.1：daily note / templates / backlinks / tag filter。

ADR-gated tracks 需先草擬 ADR-029 ~ ADR-037 才可進實作（見 `docs/tasks/blocked.md`）。

## Phase 7a Delivery Summary

- 後端：`ai.cancel` IPC、三家 provider streaming (Ollama NDJSON / OpenAI SSE / Claude SSE)、`AgentRuntime` FIFO 20 + `KnowledgeStoreArchiveSink` → `agent_archive` 表、approval timeout 事件 + `wait_for_react_approval` 寫回 `"approval_timeout"`、approve(remember) 短路下次同工具 approval。
- 前端：streaming token append、`<ElapsedTimer>`、cancel button（chat + agent running/planning）、`react-markdown` + `rehype-highlight`、`<ErrorCard>` + CTA、hover Copy/Regenerate、`useTextareaAutosize`、`useLocalHistory` ↑/↓ recall、audit default 折疊 setting、`<ApprovalCard>` 含 `<CountdownPill>` + remember checkbox + `<ApprovalSummary>` per kind + Show raw。
- Settings 新增：`ai.stream_enabled`、`agent.show_audit_by_default`、`agent.run_history_cap`、`agent.approval_timeout_secs`。
- 測試 242/243 (Phase 7a 新增 +9，1 個 pre-existing failure 不變)；clippy `-D warnings` / npm lint / tsc 全清。

## Known Risks

- If lifecycle refactor lands without measurement scripts, memory goals cannot be validated.
- If preview/read boundaries are mixed, private file exposure risk increases.
- If docs are not synchronized during refactor, task-state drift will reappear.
