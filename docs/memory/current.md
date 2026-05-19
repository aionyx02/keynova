---
type: working_memory
status: active
priority: p0
updated: 2026-05-19
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

- 2026-05-19 Bug B 真根因 round 3：使用者實測 + file.delete trace 證明 launcher 真有 trash。「桌面內容沒刪除」拆兩個獨立子問題：
  - **B1（非 launcher bug）**：Windows Explorer desktop view 不主動 refresh（`SHCNE_DELETE` 被 OneDrive / Defender / 多 Explorer instance 吞或延遲）。檔案在 Recycle Bin，按 F5 即更新。修法：`CommandPalette.tsx` success hint 加 `"· Press F5 on desktop if icon lingers"`，duration 1500→2500ms。
  - **B2（launcher bug）**：restore 後搜不到。`recentlyDeleted` kill-set sticky。修法：`Set<string>` → `Map<string, number>` + 30s TTL；`new RECENTLY_DELETED_TTL_MS = 30_000` 常數；`visibleResults` 過濾改用 `killTs > Date.now() - TTL`。3 處 `new Set()` clear 改 `new Map()`。
  - 檢查：tsc 清、lint 清（Date.now in filter eslint-disable）；backend 未動。

- 2026-05-19 Bugfix round 2 — Bug A real root cause + Bug B UX:
  - **Bug A**：先前定位 IME composition 不準。使用者澄清英文打字也會被收起 → 是 WebView2 transparent window 對任何 keystroke 都會 emit 短暫 `Focused(false)` blip。修法：`window.rs` grace `400ms→1500ms`、`dispatch.rs` keep-open guard TTL `600ms→2000ms`；frontend `CommandPalette.tsx` 拔 onComposition* listener，改 input `onFocus` + `onKeyDown` 200ms throttle 主動 renew guard。任何使用者互動都把 guard 推到未來 → backend sleep 完檢查必有效。
  - **Bug B**：→ → Enter 後 row 沒消失的真根因不是後端 trash silent fail（已由 9dfd15b verify_path_removed 防住），而是 2-stage gate 對鍵盤使用者來說 visual 太弱。修法：`SecondaryActionMenu.tsx` focused destructive row 在 armed 時 label 動態改為 `⚠ Confirm <X>? Enter again · Esc cancel`，row 加 `animate-pulse + border-l-4 + bg-red-900/60`；移除舊 confirm banner（一個強訊號勝過兩個）。`CommandPalette.tsx` delete/rename/move 三個 preview hint 統一加 ⚠ 與 "Enter again"，duration 3000→4000ms。
  - 檢查：cargo test 350/351（pre-existing nvim 不變）、clippy `-D warnings` 清、tsc 清、lint 清（一處 `react-hooks/purity` 在 event handler 內的 `Date.now()` inline disable）。
  - 待使用者實測 30s 英文 / 30s IME / 點別處 1.5s 自動收起 / Delete row pulse + hint 兩條路徑。

- 2026-05-18 Secondary action menu size fix: fixed `SecondaryActionMenu` at `390 x 360`, made its action body scrollable with focus auto-scroll, and reserved stable CommandPalette height while open so lower actions like `Delete` remain reachable. `npx tsc --noEmit` and `npm run lint` pass.
- 2026-05-18 Search stale-path bugfix: `SearchManager::file_results_for_backend` filters missing or type-mismatched file/folder paths before display for APP_CACHE, Tantivy, Everything, and native indexer results. Added regression tests for stale-path removal and post-filter limit behavior; focused test + clippy pass. Full cargo test still has the known pre-existing `note_lazyvim_missing_nvim_returns_inline_guidance` failure.
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

- 2026-05-18 LAUNCH.2 Slice 1 (A + B) 交付：
  - `handlers/search.rs` 加 `strip_global_prefix` (pure helper) + `resolve_workspace_filter` + `apply_workspace_filter` (file/folder/app 受 workspace.project_root 限制；`:global` 前綴繞過；command/note/history/model 不受影響)。Sync 與 stream 兩條路徑都套用；`StreamWorkerRequest` 加 `workspace_root` 欄位。
  - `shortcuts.rs` 加 `Ctrl+Alt+0` (預設) workspace cycle hotkey — spec 原訂 `Ctrl+Alt+W` 但已被 mouse cursor up 佔用，文檔已註明。emit 獨立 `workspace-cycled` event；前端 listener 清 query/results（與 `workspace-switched` 還原 query 行為區隔）。
  - `default_config.toml` + `settings_schema.rs` 加 `hotkeys.workspace_cycle`。
  - 5 個新 backend tests；cargo test 344/345 (pre-existing nvim 不變)；clippy/lint/tsc 全清。
  - C per-workspace quick actions 留下批（需 schema 變更 + UI）。

- 2026-05-18 ONBOARD.1 Slice 1 (A + B + C) 交付：
  - 新元件 `src/components/OnboardingTour.tsx` (4 步 modal + localStorage `keynova.onboarding.completed`) + `src/components/CheatsheetOverlay.tsx` (4 section 靜態鍵位列表)。
  - Backend 新 `OnboardCommand` builtin (`/onboard`)；前端 `execCommand` 攔截 name="onboard" → `resetOnboarding()` + reopen。
  - CommandPalette empty-state CTA：query 非空 + 0 raw results 顯示 4 個 chip (Create note / `/help` / `/setting` / Replay `/onboard`)。
  - `?` 鍵在 query 為空時觸發 cheatsheet（避免與 search 字符衝突）。
  - Conditional mount (`{open && <Component … />}`) 取代 prop+effect 防 setState-in-effect lint。
  - cargo 339/340 (pre-existing nvim 不變)；clippy / lint / tsc 全清。
  - D (re-engage usage tracking) + E (hotkey 衝突偵測) 留下一批。

- 2026-05-18 UTIL.2.J killport 交付（UTIL.2 group 結案，整段搬 completed.md）：
  - 新模組 `src-tauri/src/core/process_lookup.rs`：`ProcessInfo` struct、`find_process_by_port` 跨平台 (Windows netstat+tasklist / Unix lsof)、`kill_pid` (taskkill / kill -9)、3 個 pure parsers 抽到 module 頂層測。
  - `KillPortCmd` (`handlers/dev_utils_cmd.rs`)：兩段式 confirm — `killport <port>` preview、`killport <port> kill` 第二次 lookup 後執行（TOCTOU 防護）。
  - 安全邊界：launcher-only、無 IPC route、無 agent tool、user-initiated；同 LAUNCH.1.B 二段式 confirm pattern；不需 ADR。
  - 測試：15 個新（core::process_lookup 12 + KillPortCmd 3）；cargo test 339/340（pre-existing nvim 不變）；clippy clean。

- 2026-05-18 UTIL.2 Dev Utilities Slice 1 (A–I) 交付（14 個 builtin commands）：
  - 新模組 `src-tauri/src/core/dev_utils.rs`：uuid_v4、nanoid、generate_password (sym/alnum/alpha)、hash_text (md5/sha1/sha256/sha512)、b64_encode/decode、url_encode/decode、json_pretty/minify、regex_test、jwt_decode (含 exp hint)、color_convert (hex/rgb/hsl)、cron_explain (5/6/7 欄 + 下 5 次 fire)。
  - 新模組 `src-tauri/src/handlers/dev_utils_cmd.rs`：14 個 BuiltinCommand wrappers (UuidCmd/NanoidCmd/PwCmd/HashCmd/B64enc/B64dec/Urlenc/Urldec/Json/Jsonm/Regex/Jwt/Color/Cron) 全部 inline 結果。
  - `state.rs::build_builtin_registry` 註冊 14 個指令；皆無 state，與 CalCommand 同層。
  - Cargo deps：`base64`、`regex`、`md-5`、`sha1`、`cron`、`urlencoding`、`rand`。
  - 測試：31 個新增（dev_utils 22 + dev_utils_cmd 9）；cargo test 324/325（pre-existing nvim test 不變）；clippy clean。
  - UTIL.2.J killport 留下一批處理。

- 2026-05-18 UTIL.1 Calculator++ offline parts 交付：
  - `CalculatorManager`：新增 try_currency_conversion (offline 18 currencies, `(offline rate, snapshot 2026-05)` suffix)、try_date_arithmetic (chrono-based, today/tomorrow/yesterday/+−N units/weekday/date-diff)、convert_unit 加 12 個 volume 單位。
  - `today_override: Option<NaiveDate>` 加入 manager 給 date 測試固定 reference 日期。
  - `chrono = "0.4"` 已是既有依賴，無新 crate。
  - 17 個新測試（5 volume + 4 currency + 8 date）；cargo test 293/294（pre-existing nvim test 不變）；clippy clean。
  - ADR-038 `docs/adr/0038-currency-online-rates.md` 草擬完成（status: 提議），規範 exchangerate.host / open.er-api 雙 provider、24h cache、TLS、無使用者資料外流；登錄到 `decisions.md` + `blocked.md`。
  - UTIL.1 group **未搬 completed.md**：UTIL.1.B-online 待 ADR-038 接受。

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

LAUNCH.2 Slice 1 (A + B) 已於 2026-05-18 完成；C per-workspace quick actions 留下一批。ONBOARD.1.D/E 同樣待做。

待使用者選擇下一個起手 phase；其餘無 ADR 阻擋入口：

- Phase 12 — LAUNCH.2.C per-workspace quick actions（schema + UI）。
- Phase 8c 尾 — ONBOARD.1.D Re-engage prompt + .E hotkey 衝突偵測。
- Phase 12 — NOTE.1：daily note、templates、backlinks `[[wiki]]`、tag filter、note 全文接主 launcher、HTML/PDF 匯出 (6 子任務)。

ADR-gated tracks 需先審批 ADR-029 ~ ADR-038 才可進實作（見 `docs/tasks/blocked.md`）。UTIL.1.B-online 待 ADR-038 接受。

## Phase 7a Delivery Summary

- 後端：`ai.cancel` IPC、三家 provider streaming (Ollama NDJSON / OpenAI SSE / Claude SSE)、`AgentRuntime` FIFO 20 + `KnowledgeStoreArchiveSink` → `agent_archive` 表、approval timeout 事件 + `wait_for_react_approval` 寫回 `"approval_timeout"`、approve(remember) 短路下次同工具 approval。
- 前端：streaming token append、`<ElapsedTimer>`、cancel button（chat + agent running/planning）、`react-markdown` + `rehype-highlight`、`<ErrorCard>` + CTA、hover Copy/Regenerate、`useTextareaAutosize`、`useLocalHistory` ↑/↓ recall、audit default 折疊 setting、`<ApprovalCard>` 含 `<CountdownPill>` + remember checkbox + `<ApprovalSummary>` per kind + Show raw。
- Settings 新增：`ai.stream_enabled`、`agent.show_audit_by_default`、`agent.run_history_cap`、`agent.approval_timeout_secs`。
- 測試 242/243 (Phase 7a 新增 +9，1 個 pre-existing failure 不變)；clippy `-D warnings` / npm lint / tsc 全清。

## Known Risks

- If lifecycle refactor lands without measurement scripts, memory goals cannot be validated.
- If preview/read boundaries are mixed, private file exposure risk increases.
- If docs are not synchronized during refactor, task-state drift will reappear.
