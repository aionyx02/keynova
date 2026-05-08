# Keynova Tasks

> Compact task board. Completed work is summarized; remaining work is grouped into Phase 5 sub-branches ordered by priority.
> Update `memory.md` and `decisions.md` when a batch changes architecture or runtime behavior.

## Current Baseline

- [x] Phase 0–3.8: Tauri/React/Rust scaffold, CommandRouter/EventBus, launcher UI, app/file search, terminal, settings, hotkeys, mouse/system, AI/translation/workspace/note/calculator/history/model features.
- [x] BUG-1 to BUG-15: All user-facing fixes including first-open lazy panel sizing/scrollbar refresh.
- [x] Phase 4 Batches 1–5: ActionRef/ActionArena, UiSearchItem, schema-driven settings, Workspace Context v2, Knowledge Store DB worker, Plugin manifest/permissions, Agent runtime/approval/audit/memory, Tantivy persisted index, workspace session binding, search stream/chunked progressive results, automation executor, app struct split.
- [x] Batch 4/4.5 Agent: transparency UI, read-only tools, approval state machine, prompt audit/context budget, medium/high-risk action scaffolding, SearXNG/Tavily web search, DuckDuckGo fallback, ReAct safety foundation (schemars, AgentToolSpec, observation redaction, SystemIndexer, ADR-026/027).
- [x] Search ranking: SearchPlan per-provider quotas, sort_balanced_truncate, WSL home enumeration, OneDrive/Dropbox dynamic discovery, 28 Rust tests passing.
- [x] /note LazyVim: Terminal(TerminalLaunchSpec), NoteManager path helpers, nvim detection, PTY direct launch, Escape policy, LazyVim config bootstrap, git-ignored .keynova/.

Last full verification baseline: `npm run build`, `npm run lint`, `cargo test`, `cargo clippy -- -D warnings`, `git diff --check`.

---

## 建議執行順序

```
5.1.A → 5.1.B → 5.8
  → 5.2.A → 5.2.B
  → 5.3.A → 5.3.B → 5.3.C
  → 5.4
  → 5.5.A1 → 5.5.A2 → 5.5.B1 → 5.5.A3/A4 → 5.5.B2 → 5.5.B3 → 5.5.C → 5.5.E → 5.5.F → 5.5.G/H
  → 5.7.A → 5.7.B → 5.6 → 5.9+
```

5.5 是長期主線，每個子步驟獨立可測、分批 merge，不等全部完成才合併。

---

## Phase 5.1 — P0 Critical Bug Fixes

> 這兩個 bug 修完才開新功能。範圍小、影響明確。

### 5.1.A — Agent Frontend Run Ordering Bug ✓

- [x] 修 `useAgent.ts` `upsertRunChronologically()`：新 run 改為 prepend（`[run, ...prev]`），確保 `runs[0]` 永遠是最新 run。
- [x] 確認 `AiPanel.tsx` 讀 `agent.runs[0]` 為最新，且 `command_result` 能正確交付給 `onRunCommandResult()`。
- [ ] Regression 三種情境（需在真實 app 手動驗收）：
  - 第一次 Agent action → 開 note panel
  - 第二次 Agent action → 開 setting panel
  - 第三次 Agent action → 開 terminal result
  - 驗證 `runs[0]`、`deliveredResultRef`、`onRunCommandResult()` 三者不互相卡住。

### 5.1.B — AI Chat Message Retain Bug ✓

- [x] 快速修：`AiManager::chat_async()` 失敗時改用 `rposition()` 找最後一筆符合 `role=user && content=prompt` 的訊息並 `remove()`，不使用 `retain()` 全量刪除。
- [ ] 後續正規化（非本 batch）：為每次 message 加 `request_id`；本次只做最小修正。
- [ ] 單元測試：用相同 prompt 送兩次，第二次失敗，確認 history 只少一筆（需整合測試環境）。

---

## Phase 5.8 — P2 Note Name Robustness（提前至 5.1 後）✓

> 改動量極小、確定性高，插在搜尋和路徑修正之前。

- [x] 新增 `validate_note_name()`：sanitize 後結果為空時回傳 `Err`（保護 `get`/`create`/`save`/`delete`/`rename`）。
- [x] 加 reserved name 檢查：拒絕 `CON`/`PRN`/`AUX`/`NUL` 及 `COM1`–`COM9`/`LPT1`–`LPT9`（case-insensitive，對 sanitized 名稱檢查）。
- [x] 單元測試 4 個：空輸入、純空白、reserved names、valid names（含 `CONSOLE`/`COM10`/`lpt` 邊界確認）。
- 說明：純符號輸入（`"!!!"`）sanitize 為 `"___"`，屬合法名稱不拒絕；`.` 和 `..` 同理 sanitize 為底線。

---

## Phase 5.2 — P1 Search Stream Fairness & Diagnostics

### 5.2.A — Frontend Stream Merge Quota ✓

- [x] 新增 `applySourceQuotas()` 函式，caps 與後端 `sort_balanced_truncate` 常數一致（app/command/note:8, history:12, model:6, file:unbounded）。
- [x] `mergeSearchResults()` 改用 `applySourceQuotas(sortSearchResults(merged), limit)` 取代 `.slice(0, limit)`。
- [x] 後端 `run_stream_worker` 重構：不再發送 incremental chunks；在所有 provider 完成後，combine first_batch + worker items → `sort_balanced_truncate` → 單一 `replace: true, done: true` final chunk。
- [x] 後端移除 `DEFAULT_CHUNK_SIZE` 常數和 `SearchPlan::internal_limit` 欄位（不再使用）。
- [x] `SearchChunkPayload` 加 `replace?: boolean`；chunk listener 遇到 `replace: true` 直接 `setResults(applySourceQuotas(sortSearchResults(...)))`。
- [ ] 驗證（需真實 app）：app/command/history 已滿 display limit 時，file results 仍可見。

### 5.2.B — Search Stream Diagnostics ✓

- [x] 後端新增 `SearchChunkDiagnostics` struct：`elapsed_ms`、`timed_out`、`file_cache_entries`、`tantivy_index_entries`、`everything_available`、`pre_balance_count`、`returned_count`、`fallback_reason`。
- [x] diagnostics 包進 `search.results.chunk` payload（`replace:true` 的 final chunk，不另開事件）。
- [x] 前端 `fileDiagnostics` state；footer 顯示可讀訊息：
  - `File search: provider timed out after 800ms`
  - `File search: Tantivy index empty, using cache fallback`
  - `File search: Everything unavailable, using cache fallback`
  - `File search: N shown, M hidden by display limit`
- [ ] 在真實 app 中確認 live backend readiness（需手動驗收）。

---

## Phase 5.3 — P1 Cross-Platform Config Paths

### 5.3.A — ConfigManager Path Fix ✓

- [x] 新增 `src/platform_dirs.rs`：`keynova_config_dir()`（`dirs::config_dir()` + `"Keynova"`）和 `keynova_data_dir()`（`dirs::data_dir()` + `"Keynova"`）。
- [x] `ConfigManager::user_config_path()` → `keynova_config_dir().join("config.toml")`，移除 APPDATA env var 直讀。
- [x] 加 `dirs = "5"` 到 `Cargo.toml`；`pub mod platform_dirs` 加入 `lib.rs`。

### 5.3.B — 對齊所有資料路徑消費者 ✓

- [x] `NoteManager::new()` → `keynova_data_dir().join("notes")`
- [x] `knowledge_store.rs` `default_db_path()` → `keynova_data_dir().join("knowledge.db")`
- [x] `tantivy_index.rs` `default_index_dir()` → `keynova_data_dir().join("search/tantivy")`
- [x] `system_indexer.rs` 呼叫 `resolve_index_dir(None)` 自動受益（不需另改）。
- [ ] Regression（需手動驗收）：從非預期 cwd 啟動 app，確認 config/notes/history 寫入正確 OS 目錄。

### 5.3.C — Legacy Path Migration

- [ ] 啟動時偵測：若新 OS config dir 尚無資料，但舊路徑（`./Keynova/` 或 cwd-based APPDATA fallback）存在，執行以下流程：
  - `config.toml`：自動複製一次。
  - `notes/`、`history/`、db、search index：提示使用者或靜默複製（根據資料量決定）。
  - 寫入 `migration_complete` marker，避免重複執行。
- [ ] 確保升級後使用者筆記、history、設定不會憑空消失。

---

## Phase 5.4 — P1 Default AI Provider & First-Run UX

- [ ] 修改 `default_config.toml`：預設 provider 改為 `"ollama"`，`model = "qwen2.5:7b"`（符合 local-first ADR）。
- [ ] First-run / 每次 `/ai` 開啟時檢測：
  1. Ollama daemon 是否 reachable（`http://localhost:11434/api/tags`）。
  2. 目標 model 是否已 pull（比對 tags list）。
  3. 若 daemon 不可達或 model 不存在，`AiPanel` 顯示 setup card：
     - Step 1：安裝 Ollama（附連結）
     - Step 2：`ollama pull qwen2.5:7b`（或根據 RAM 推薦 `qwen2.5:1.5b` / `llama3.2:1b`）
     - Step 3：或切換到 OpenAI / Claude（填 API key）
- [ ] RAM-based 推薦：`model_manager` 已有硬體推薦邏輯，first-run 可直接呼叫，推薦 model 顯示在 setup card。
- [ ] 若 provider = "claude" 且 `api_key = ""`，同樣觸發 setup card，不在送出後才報錯。
- [ ] 更新 `README.md` local-first 段落，反映新預設值。

---

## Phase 5.5 — P0 Agent ReAct Loop Wiring（長期主線，分批 merge）

> 不一次做完。每個子步驟獨立可測，通過 `cargo test` / `npm run build` 後即可 merge。

### 5.5.A1 — Provider Trait / Interface 定義

- [ ] 在 `managers/ai_manager.rs` 定義 provider trait：`chat_with_tools(messages, tools) -> ToolCallResponse`。
- [ ] `ToolCallResponse`：`{ tool_calls: Vec<AiToolCallRequest>, final_text: Option<String> }`。
- [ ] 移除 `AiToolDefinition`、`AiToolCallRequest`、`AiToolObservation`、`AiToolTurn` 上的 `#[allow(dead_code)]`。

### 5.5.A2 — FakeToolCallProvider（僅測試用）

- [ ] 實作 `FakeToolCallProvider`：可設定「本輪回傳 tool call X」或「本輪回傳 final text Y」。
- [ ] 用於 5.5.B1 skeleton 的整合測試，不進 production 路徑。

### 5.5.B1 — ReAct Loop Skeleton（FakeProvider + keynova.search）

- [ ] 在 `core/agent_runtime.rs` 實作最小 ReAct 迴圈：filtered context → inject schemas → call provider → parse tool calls → execute `keynova.search` → redact observation → append → 下一輪。
- [ ] 停止條件：final text / cancel / timeout / max_steps。
- [ ] 每步向前端 emit `agent.step { step, tool_name, status, observation_preview }`。
- [ ] 所有測試用 FakeProvider；此步驟不接真 LLM。

### 5.5.A3 — OpenAI-Compatible Provider Tool-Call Parsing

- [ ] 實作 OpenAI-compatible `chat_with_tools()`：對應 `tools` + `tool_choice` request fields；解析 response choice 中的 `tool_calls`。
- [ ] Provider 不支援 tool calls 時回傳 typed `UnsupportedToolCallError`。

### 5.5.A4 — Ollama Provider Tool-Call Support

- [ ] 實作 Ollama `chat_with_tools()`：對應 function-calling format（若模型支援）。
- [ ] 不支援時回傳 `UnsupportedToolCallError`，觸發 5.5.C offline fallback。

### 5.5.B2 — 加入 filesystem.read / filesystem.search 工具

- [ ] 在已通過的 ReAct loop skeleton 上加入 `filesystem.read`（read-only preview）和 `filesystem.search` 工具。
- [ ] 整合 SystemIndexer 作為 `filesystem.search` 後端。

### 5.5.B3 — 加入 approval-gated git.status

- [ ] 加入 `git.status` tool spec，標記為 approval-required。
- [ ] 整合既有 approval gate；確認未批准前不執行。

### 5.5.C — Move Heuristics to Offline Fallback

- [ ] 在 `handlers/agent.rs` 中，將 `direct_local_answer()`、`sources_for_prompt()`、`detect_planned_action()` 移入 `provider_supports_tool_calls == false` 或 `agent.mode = "offline"` 的分支。
- [ ] 正常模式：LLM 決定 tool calls；heuristics 僅在 provider 不可用或模型不支援 function calling 時啟動。

### 5.5.D — Platform Command Sandbox（Research Only, Blocked）

- [ ] 探索 Linux `bwrap`/seccomp、Windows restricted token/job、macOS `sandbox-exec` 的可行性。
- [ ] **不進 main，不暴露 generic shell tool**。保持 research branch 狀態，等 ADR-027 批准後才進入 product 路徑。

### 5.5.E — Persist ReAct Audit

- [ ] 將每個 ReAct step 持久化到 Knowledge Store `agent_audit_logs`：`request_id`、tool schemas、args preview、approval decision、observation summary、redacted sources、final answer、failure/cancel reason。

### 5.5.F — Frontend Agent Run View

- [ ] 更新 `AiPanel` run view：pending approval 指示器、approved/running/completed/denied tool 狀態、stdout preview、observation count、redaction notices、final grounded answer。

### 5.5.G — ReAct Regression Tests

- [ ] Provider tool-call parsing、schema generation、loop max-step stop。
- [ ] Cancellation、one-shot approve/reject、unsafe-command denial。
- [ ] stdout/stderr 截斷、provider timeout、web-query redaction、stale OS index fallback。

### 5.5.H — ReAct Manual Validations

- [ ] JSON file find、read-only preview、web query。
- [ ] Denied shell request、git status command（approval-gated）。
- [ ] Project search via Everything/Tantivy/SystemIndexer。
- [ ] Stale index warning、rejected-approval self-correction、final grounded answer。

---

## Phase 5.6 — P1 Agent Quality & Existing Regressions

- [ ] Agent planning quality：no-action runs 解釋 Keynova 下一步能做什麼；減少意外的 panel/action matches；high-risk actions 維持在 explicit approval 後方。
- [ ] Regression tests：cancellation/reject lifecycle、one-shot approval、tool error surfacing、prompt budget truncation、private architecture denial、secret redaction、web-query redaction。
- [ ] External agent integration contract：定義最小 provider 介面，允許 plan + tool requests + Keynova-safe action proposals，且不繞過 approval/audit。
- [ ] Manual validation script：local search、feature search、private architecture denial、note draft approval、terminal approval、rejected approval、disabled web search、action payload handoff。

---

## Phase 5.7 — P1/P2 Search & Index Alignment

### 5.7.A — Agent/Launcher Tantivy Index Unification（可提前至 5.3 後）

- [ ] 修 `system_indexer.rs` `search_system_index()`：接收來自 config 的 `tantivy_index_dir`，不再呼叫 `resolve_index_dir(None)`。
- [ ] 引入 `SystemIndexerConfig { tantivy_index_dir, roots, timeout, max_visited }`，從 `SearchManager` config 傳入，確保 Agent 和 launcher 使用同一個 index。
- [ ] 提前原因：Agent 和 launcher index 不同會讓搜尋診斷結果混亂，建議在 5.3 路徑統一後即執行。

### 5.7.B — Non-Windows Launcher File Search（建議升至 P1）

- [ ] 修 `SearchManager::file_results_for_backend()` 在非 Windows 上改用 `SystemIndexer`（macOS `mdfind`、Linux `plocate`/`locate`、`ignore` fallback），不再回傳 `Vec::new()`。
- [ ] Launcher 和 Agent 在 Linux/macOS 上產生一致的 file results。
- [ ] 注意：維持跨平台承諾需要此項；若主要開發環境仍為 Windows，可暫保持 P2。

---

## Phase 5.9 — Plugin & WASM Runtime

- [ ] WASM runtime proof-of-concept。
- [ ] Host Functions API：`log`、`get_setting`、`search_workspace`、`read_clipboard`、`http_fetch`、`run_action`。
- [ ] Plugin action/search provider 註冊。
- [ ] Plugin settings schema。
- [ ] Plugin audit log 寫入 Knowledge Store。
- [ ] Hot reload。

---

## Phase 5.10 — Future Features & Maintenance

- [ ] Smart Clipboard：classify code/url/path/email/json/error log/command/markdown/image 並提供 actions。
- [ ] Dev Workflow Pack：git status/branch、cargo test、npm build、open project、search symbol、explain compiler error、create commit message。
- [ ] Command chaining：`/search error | ai explain | note append`。
- [ ] Universal Command Bar：跨 app/file/note/history/command/action，AI fallback。
- [ ] 保持快捷鍵文件與 `README.md` 同步。
- [ ] 維護 panel first-open/ESC/resize regression checklist。
- [ ] 新增 provider/action/search source metadata 文件供 user-facing diagnostics 使用。
- [ ] 手動驗收 /note LazyVim：`/note`、`/note lazyvim`、`/note lazyvim foo`、Vim 內 Esc、terminal exit shortcut。

---

## Blocked

- Phase 5.5.D（platform sandbox）— Research branch only。等待 ADR-027 批准後才進 product；generic shell tool 不在此前解封。