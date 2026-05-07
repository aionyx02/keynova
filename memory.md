# memory.md — AI 工作記憶

## 2026-05-07 Update - ReAct Tool Safety Foundation

- Batch 4.6 started with the safety/type foundation before provider-loop wiring.
- Added `schemars` and generated Agent tool parameter/result schemas from Rust structs. `AgentToolSpec` now carries LLM-safe names, descriptions, generated schemas, approval policy, visibility policy, timeouts, result limits, and observation redaction policy.
- Added `core::agent_observation` with hard character/line limits, likely-secret line redaction, and head+tail preservation so large stdout/file/search observations do not enter LLM context unbounded.
- Added provider-neutral tool-call contract types in `AiManager`; Agent execution must use the same selected AI provider/model as AI Chat, with OpenAI-compatible and local Ollama declared as first ReAct tool-call targets.
- Generic `execute_shell_command` / `execute_bash_command` is intentionally absent from the default tool registry. A typed `git.status` spec is present but approval-gated and blocked from the legacy direct `agent.tool` path.
- README has been rewritten to reflect the current local-first AI/Agent architecture, selected-provider behavior, safety boundary, and development commands.
- Added `SystemIndexer` for Agent filesystem search: Windows Everything/Tantivy, macOS `mdfind`, Linux `plocate`/`locate`, and bounded parallel `ignore` fallback with diagnostics.
- Agent web search now treats DuckDuckGo HTML as explicit best-effort fallback. Structured paths are SearXNG JSON and Tavily API; default `agent.web_search_provider` is disabled until configured.
- ADR-026 defines the selected-provider ReAct loop contract, and ADR-027 blocks generic shell tools until real platform sandboxing exists.
- ADR-023 records this foundation. Verification passed: `cargo test`, `cargo clippy -- -D warnings`, `git diff --check`.

## 2026-05-07 Update - Agent Runtime Batch 4

- Batch 4 agent runtime now uses an explicit approval state machine with typed planned actions instead of reusing generic launcher actions for risk handling.
- `agent.start` builds a prompt audit with context budget, visibility filtering, filtered-source metadata, and secret/private-architecture redaction before any agent action is proposed.
- Medium-risk approvals can now open panels, create note drafts, update setting drafts, and run allowlisted safe built-in commands. High-risk approvals can launch terminal commands and scaffold file/system/model actions behind explicit confirmation.
- Agent runs can return a `BuiltinCommandResult` payload so the AI panel can hand approved note/setting/system/model/terminal intents back to the launcher UI.
- Knowledge Store schema version is now 3, adding `agent_audit_logs` and `agent_memories`. Long-term memory is opt-in through `agent.long_term_memory_opt_in`.
- `NoteEditor` now respects `initialArgs` for both plain note selection and agent draft payloads. `SettingPanel` now respects draft key/value payloads from the agent.
- Verification passed: `cargo test`, `cargo clippy -- -D warnings`, `npm run lint`, `npm run build`.

## 2026-05-07 Update - Search Runtime Batch 2/3

- Search now stores a real on-disk Tantivy index with `name`, `path`, `kind`, and folder metadata. Windows rebuild refreshes the existing file cache, snapshots it, and writes the persisted index under `[search].index_dir` or `%APPDATA%\Keynova\search\tantivy`.
- `search.backend` diagnostics now include Tantivy document count and index directory. `auto` can select Tantivy when a persisted index is present, with app-cache fallback if the index is empty or unavailable.
- Search results still carry lightweight `icon_key`, but the selected result now lazy-loads an SVG icon asset through `search.icon`; the frontend caches icon assets by key.
- Clipboard history entries record the active workspace id. History search ranks pinned entries first and boosts entries from the current workspace.
- Recent/frequent search selection memory is tracked in-process through `search.record_selection`, boosting launched results by recency and frequency.
- Knowledge Store connections now manage `PRAGMA user_version = 2`, create `schema_migrations`, and backup existing older DBs into a sibling `backups/` directory before migration.
- Verification passed: `cargo test`, `npm run lint`, `npm run build`, `cargo clippy -- -D warnings`, `git diff --check`.

## 2026-05-07 Update - /note LazyVim Terminal Launch

- `/note` default behavior remains the built-in note panel.
- `/note lazyvim`, `/note lazyvim <note-name>`, and `/note lazyvim --path <absolute-or-relative-path>` now return a terminal-backed command result when `nvim` is available.
- `CommandUiType` now supports `Terminal(TerminalLaunchSpec)`; the frontend `BuiltinCommandResult` union mirrors that contract.
- `NoteCommand` owns `NoteManager` + `ConfigManager` references so it can resolve note paths and read `notes.lazyvim_command`.
- `NoteManager` exposes `notes_root()`, `resolve_named_note()`, and `create_parent_dirs_for_file()`.
- `TerminalManager::create_pty_with_command` starts direct PTY commands without going through the default shell, and `terminal.open` accepts an optional `launch_spec`.
- `TerminalPanel` recreates PTYs when `launch_id` changes. Editor sessions keep Escape for Vim and exit via `Ctrl+Shift+Q` or `Ctrl+Alt+Esc`.
- LazyVim integration follows official docs: detect a LazyVim starter config marker, use `notes.lazyvim_config_dir` when configured, and auto-bootstrap missing starter config into project-local `.keynova/lazyvim/config/keynova-lazyvim`.
- LazyVim terminal env now points `NVIM_APPNAME`, `XDG_CONFIG_HOME`, `XDG_DATA_HOME`, `XDG_STATE_HOME`, and `XDG_CACHE_HOME` at project-local `.keynova/lazyvim/*` paths so generated config/plugin/cache content is deleted with the project. `.keynova/` is git-ignored.
- Verification passed: `cargo test`, `npm run lint`, `npm run build`, `cargo clippy -- -D warnings`, `git diff --check`. Manual app validation is still pending.

## 2026-05-06 Update - Tasks Compact + Automation Executor

- `tasks.md` is now a compact batch board instead of a long historical checklist. Completed work is summarized; remaining Phase 4 work is grouped into batches.
- Phase 4.6 action chain executor is implemented through `automation.execute`, which parses TOML workflow source, runs each action through the existing dispatch boundary, records per-action output/error, stops on the first failure, and rejects recursive `automation.execute`.
- Phase 4.0 app split is complete: `lib.rs` now only declares modules and delegates `run()` to `app::bootstrap`; state, dispatch, control server, watchers, shortcuts, tray, and window setup live in `src-tauri/src/app/*`.
- Phase 4.1/4.4 search runtime now supports stream mode: `search.query` can return a fast first batch, then publish `search.results.chunk` events for later pages. The frontend listens to `search-results-chunk`, merges chunks by source/path, and cancels stale requests.
- Search provider timeout is in place for slow file providers. File results are loaded in the background worker; the first visible batch uses app/command/note/history/model sources only.
- `search.metadata` lazily loads selected file/app metadata and small text previews. Icon payloads remain lightweight `icon_key` only; real icon asset lazy loading is still pending.
- The `tantivy` backend preference now has an offline file-provider path backed by the rebuilt file cache. A real persisted Tantivy schema/on-disk index is still pending.
- Phase 4.3 workspace binding started: terminal open/close updates `terminal_sessions`, note get/save/create/rename/delete updates `note_ids`, and AI chat request ids are recorded in `ai_conversation_ids`.
- Next high-value batch choices: workspace-weighted history/search ranking and recent/frequent cache, then real persisted Tantivy index/icon asset lazy load.

> 每次開新 session，AI 必須先讀此檔案，確認現在進度，再繼續工作。
> 完成任何重要步驟後，請更新此檔案。

## 當前狀態

- **進度**：Phase 4 第一輪核心 foundation 已完成；lazy panel 首次開啟視窗高度同步已修正；Phase 2 搜尋 backend debug/config/rebuild/test 補齊；Knowledge Store batch writes + shutdown flush 已補上；Plugin loader 已可讀取/驗證本機 manifest；Agent read-only tools 已可執行 `keynova.search` / SearXNG `web.search`。
- **上次完成**：補齊 Agent read-only tool runtime 與搜尋 cancellation：`agent.tool` 可呼叫 `keynova.search` / `web.search`，`agent.start` 會在需要時帶入本機 grounding sources；`keynova.search` 搜尋 workspace、command、settings schema、model、notes、history 並套用 visibility filter；`web.search` 支援 SearXNG 且拒絕 private architecture / secret query；`search.cancel` 與 backend generation check 可丟棄 stale provider results。
- **下一步**：補 Phase 4 尚未完成的重型 runtime：`lib.rs` 拆成 `app/*`、search chunk streaming、Tantivy provider、Agent approval/audit/memory、WASM runtime/hot reload。

## 已確認的技術選擇

| 面向 | 選擇 | 原因 |
|------|------|------|
| 前端框架 | React 18 + TypeScript | 生態成熟，Tauri 官方支援 |
| 狀態管理 | Zustand | 輕量，無 boilerplate |
| 樣式 | Tailwind CSS | 快速迭代，不需要 CSS 命名 |
| 後端語言 | Rust + Tauri 2.x | 包體積、效能、安全性 |
| 搜尋引擎 | tantivy（跨平台）+ Everything IPC（Windows 選配）| 雙後端自動回退，Windows 用戶可加速 |
| 終端模擬 | xterm.js + PTY | 標準方案，跨平台支援 |
| 打包工具 | Vite | Tauri 官方推薦 |
| IDE | JetBrains RustRover | Rust 原生支援，內建 Cargo / Clippy 整合 |

## 已知問題 / 待解決

- Everything IPC 完整實作尚需參考官方 SDK（`everything_ipc.rs` 目前為佔位符）
- Everything SDK 的 Rust 綁定建議評估：[EverythingSearchClient](https://github.com/sgrottel/EverythingSearchClient)

## 重要教訓

- **禁止對 dev 執行 `git merge main --no-ff`**：會將 main 中已從 git 移除的私人文件（tasks.md/memory.md/CLAUDE.md 等）的刪除操作帶進 dev，導致本地工作檔案被覆蓋消失。正確做法：在 feature 分支上工作，不要將 main merge 回 dev。

## 歷史摘要（已壓縮）

- 規劃 → Phase 0 → Phase 1 完成（2026-05-01 ~ 05-02）：文件建立、Tauri scaffold、ESLint/Tailwind、CommandRouter/EventBus、handlers/managers/models/platform 骨架、Flow Launcher 風格 UI、全域滑鼠控制、Everything 搜尋架構、51 files 合併進 main
- Phase 2 規劃（2026-05-02）：tasks 設計完成；Phase 2.A 搜尋框三模式 + Phase 2.B /command Registry；feature/phase2 分支建立
- Phase 2 全部完成 + BUG-1~5 修正 + Task 1（D槽/WSL 搜尋）+ Phase 3 tasks 拆解（2026-05-03）：merge 進 main
- 私人文件還原與 BUG-6~9 工作分支建立（2026-05-03）：private docs 因 git merge 誤刪後從 git 歷史還原，BUG-6~9 加入 tasks.md，feature/bugfix-ux 建立
- FEAT-2（keynova CLI / control plane）完成；FEAT-3（args_hint）、FEAT-4（terminal CSS display）、FEAT-5（setting keyboard nav）實作完成（2026-05-03~04）
- feature/feat1-feat2-control-plane → dev/main 合併完成；BUG-11 終端 ESC keydown/keyup 修復完成（2026-05-04）
- Phase 3.0–3.8 完成與後續任務整理（2026-05-04 ~ 05-05）：/tr /ai /note /cal /history /system + workspace 3 槽位完成；BUG-12/13/14 經 stale closure/捲動根因修復；tasks.md 壓縮並建立 FEAT-6/7
- FEAT-6 完成（2026-05-05）：`/model_download` / `/model_list` 支援 AI Chat 與 Translation 分別切換模型、硬體推薦、Ollama catalog/progress、API provider 切換；後續 FEAT-7 已將 Translation 收斂為 Google only

## Session 交接紀錄（最近 5 筆）

| 日期 | 完成事項 | 遺留問題 |
|------|----------|----------|
| 2026-05-06 | Phase 2 搜尋、Knowledge Store、Plugin loader 補齊：backend preference/active 選擇可測試、`search.backend` structured debug info、搜尋框 backend badge、`/rebuild_search_index` 背景重建 file cache、`[search]` config/schema、CSP connect-src 放行、`hotkey.register` 轉 structured `not_implemented`；DB worker 補 action/clipboard batch insert 與 shutdown flush；plugin.toml/json loader 會驗證 permission deny-by-default | Tantivy provider、WASM runtime proof-of-concept 尚未接入；BUG-1/BUG-4 仍建議在真實 DevTools/視窗手動確認 |
| 2026-05-06 | Phase 4.4/4.5A：搜尋 backend generation/cancel 可丟棄 stale provider results；AgentHandler 注入 config/note/history/workspace/command/model context；新增 `agent.tool`，實作 `keynova.search` 與 SearXNG `web.search`；新增 private architecture/secret web-query redaction tests | `web.search` 預設 disabled，需設定 `agent.web_search_provider=searxng` 與 `agent.searxng_url`；尚未做 Agent approval/action phases 與 audit 寫入 Knowledge Store |
| 2026-05-06 | 修正 BUG-15：lazy panel 首次開啟後內容載入完成未觸發 Tauri window resize，導致 UI 裁切、背景殘影與外層 scrollbar；改用 ResizeObserver/MutationObserver 同步高度，並關閉 root/body overflow | 需在真實 Tauri 視窗手動確認 `/cal`、`/ai`、`/history`、`/model_list` 首次開啟皆正常 |
| 2026-05-06 | Phase 4 foundation：ActionArena/ActionRef、UiSearchItem、action.run、secondary actions、schema-driven settings、Workspace v2、KnowledgeStore DB worker、Agent mode UI/runtime skeleton、Automation/Plugin security model；新增 ai/privacy/plugin/runtime docs；build/lint/test/clippy 全通過 | 尚未完成 `lib.rs` app 模組拆分、search chunk streaming/cancellation token、Tantivy provider、Agent 真實 web/keynova tool、WASM loader/hot reload |
| 2026-05-06 | 補強 `/ai` Agent 搜尋與隱私規劃：加入 `web.search`、`keynova.search`、GroundingSource、context visibility、architecture denylist、prompt allowlist，確保專案架構可本地索引但不注入 LLM prompt | 尚未實作；需先做 context privacy 文件與 visibility filter，避免外部 LLM 看見 private architecture |

## 2026-05-06 架構邊界與修正定位索引

目的：未來修正時，直接定位「單一模組/檔案」即可，不需掃描全域程式碼。

### A. 模組邊界（Boundary）

1. 前端 UI 與互動層（React/Tauri API）
   - `/src/main.tsx`：前端入口，掛載 `<App />`
   - `/src/App.tsx`：根組件，目前以 `CommandPalette` 為主畫面
   - `/src/components/CommandPalette.tsx`：主互動面板（搜尋/命令/終端切換、結果選取、視窗焦點行為）
   - `/src/components/panel/PanelRegistry.tsx`：Panel 路由註冊中心，負責 `/ai`, `/tr`, `/cal`, `/setting` 等面板切換
   - `/src/components/AiPanel.tsx`：AI Chat 與 Agent 互動介面
   - `/src/hooks/useIPC.ts`：統一 IPC 入口，所有前端命令透過 `cmd_dispatch`
   - `/src/stores/appStore.ts`：全域狀態（query/searchResults/loading/activePanel）

2. 後端 Core 層（核心基礎設施）
   - `/src-tauri/src/core/command_router.rs`：`namespace.command` 路由分派核心
   - `/src-tauri/src/core/action_registry.rs`：ActionArena / ActionRef 系統，定義行為與其風險
   - `/src-tauri/src/core/agent_runtime.rs`：AI Agent 執行環境、Tool 呼叫、Approval Gate
   - `/src-tauri/src/core/knowledge_store.rs`：本機 SQLite 資料庫，儲存 History/Notes/Clipboard 索引
   - `/src-tauri/src/core/config_manager.rs`：Schema-driven 配置管理，支援型別安全與 UI 連動
   - `/src-tauri/src/core/plugin_runtime.rs`：WASM / Local Plugin 載入與權限檢查
   - `/src-tauri/src/core/event_bus.rs`：後端事件匯流排（broadcast）

3. 後端功能 Handler 層（命令入口）
   - `/src-tauri/src/handlers/launcher.rs`：`launcher.*` 命令入口
   - `/src-tauri/src/handlers/builtin_cmd.rs`：`/ai`, `/tr`, `/cal`, `/setting` 等內建指令註冊
   - `/src-tauri/src/handlers/ai.rs` / `agent.rs`：AI 對話與 Agent 控制命令
   - `/src-tauri/src/handlers/model.rs`：模型切換、下載管理
   - `/src-tauri/src/handlers/search.rs`：`search.*` 搜尋引擎入口
   - `/src-tauri/src/handlers/note.rs` / `workspace.rs`：筆記與工作空間管理

4. 後端 Manager 層（業務邏輯）
   - `/src-tauri/src/managers/app_manager.rs`：App 掃描、快取、模糊搜尋、啟動
   - `/src-tauri/src/managers/model_manager.rs`：Ollama 模型生命週期與硬體推薦
   - `/src-tauri/src/managers/terminal_manager.rs`：PTY session 管理與 pre-warm
   - `/src-tauri/src/managers/workspace_manager.rs`：Workspace 槽位切換與 context 管理
   - `/src-tauri/src/managers/search_manager.rs`：App + File/Folder 搜尋聚合（Everything / Tantivy）

5. 平台實作層（Platform）
   - `/src-tauri/src/platform/windows.rs`：Windows 實作（Everything IPC、輸入模擬、檔案索引）

### B. 快速搜尋關鍵字

- `ActionRef` / `UiSearchItem`：搜尋結果與行為的橋接
- `knowledge_store` / `DB worker`：資料庫非同步讀寫
- `agent.start` / `approval`：Agent 生命週期與安全閘口
- `CommandUiType::Panel`：觸發前端特定面板顯示
- `cmd_dispatch`：前後端命令橋接入口
- `FILE_CACHE`：本機檔案搜尋索引檔案
