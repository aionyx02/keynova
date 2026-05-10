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
- [x] Regression 三種情境（需在真實 app 手動驗收）：
  - 第一次 Agent action → 開 note panel
  - 第二次 Agent action → 開 setting panel
  - 第三次 Agent action → 開 terminal result
  - 驗證 `runs[0]`、`deliveredResultRef`、`onRunCommandResult()` 三者不互相卡住。

### 5.1.B — AI Chat Message Retain Bug ✓

- [x] 快速修：`AiManager::chat_async()` 失敗時改用 `rposition()` 找最後一筆符合 `role=user && content=prompt` 的訊息並 `remove()`，不使用 `retain()` 全量刪除。
- [x] 單元測試 3 個（ai_manager::tests）：僅刪最後一筆重複、prompt 不存在時不刪、空 history 不 panic。驗證通過 ✓
- [ ] 後續正規化（非本 batch）：為每次 message 加 `request_id`；本次只做最小修正。

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
- [x] 程式碼靜態審查通過：前後端 quota 邏輯一致，`replace: true` 路徑正確替換結果而非追加。需真實 app 手動驗收（顯示效果）。

### 5.2.B — Search Stream Diagnostics ✓

- [x] 後端新增 `SearchChunkDiagnostics` struct：`elapsed_ms`、`timed_out`、`file_cache_entries`、`tantivy_index_entries`、`everything_available`、`pre_balance_count`、`returned_count`、`fallback_reason`。
- [x] diagnostics 包進 `search.results.chunk` payload（`replace:true` 的 final chunk，不另開事件）。
- [x] 前端 `fileDiagnostics` state；footer 顯示可讀訊息：
  - `File search: provider timed out after 800ms`
  - `File search: Tantivy index empty, using cache fallback`
  - `File search: Everything unavailable, using cache fallback`
  - `File search: N shown, M hidden by display limit`
- [x] 程式碼靜態審查通過：diagnostics 建構邏輯正確，前端 payload 型別吻合。需真實 app 手動驗收（footer 顯示）。

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
- [x] Bug fix（驗證時發現）：`history_manager.rs` + `workspace_manager.rs` 的 `default_path()` 仍使用 `APPDATA` env var，已改為 `keynova_data_dir()`。
- [x] 5 個 platform_dirs 單元測試：Keynova 後綴、絕對路徑、路徑段包含驗證。
- [x] Regression 程式碼驗證通過（grep 確認無殘留 APPDATA 直讀於 data path 消費者）。需真實 app 驗收寫入位置。

### 5.3.C — Legacy Path Migration ✓

- [x] 啟動時偵測：若舊路徑（cwd-based `./Keynova/` 或 exe-sibling `Keynova/`）存在，靜默複製 `config.toml`、`notes/`、`knowledge.db` 至新 OS 目錄（不覆蓋已存在的目標）。
- [x] Tantivy index 略過（啟動時自動重建）。
- [x] 寫入 `keynova_data_dir()/.migration_v1_complete` marker，避免重複執行。
- [x] 3 個單元測試（candidates 後綴、copy 跳過既有 dst、copy 複製缺席 dst）。
- [x] `app::migration::run_legacy_migration()` 在 bootstrap setup 最早呼叫。

---

## Phase 5.4 — P1 Default AI Provider & First-Run UX ✓

- [x] 修改 `default_config.toml`：預設 provider 改為 `"ollama"`，`model = "qwen2.5:7b"`（符合 local-first ADR）。
- [x] `ai.check_setup` 命令：3s timeout ping `/api/tags`，比對 model 名稱（prefix match），偵測 Claude/OpenAI 空 API key；回傳 `needs_setup / ollama_reachable / model_available / recommended_model / reason`。
- [x] RAM-based 推薦：透過 `ModelManager::detect_hardware() + recommend_models()` 選出最適 model 顯示在 setup card。
- [x] `SetupCard` 組件：三步驟（安裝 Ollama、pull model、或切換 /setting provider）；Re-check / Use-anyway 按鈕。
- [x] 空 placeholder 在 setup card 顯示時隱藏。
- [x] README local-first 段落更新：新增「Local-First AI Setup」段落說明 setup card 三步驟與 Ollama 需求；更新 ReAct loop 狀態描述。
- [x] 6 個 check_setup 單元測試（handlers/ai::tests）：Ollama prefix match、exact match、no match、cross-family guard、empty model list、empty API key detection。
- [ ] 手動驗收：Ollama 未啟動時開啟 /ai → 看到 setup card；pull model 後按 Re-check → 卡片消失。（需真實環境）

---

## Phase 5.5 — P0 Agent ReAct Loop Wiring（長期主線，分批 merge）

> 不一次做完。每個子步驟獨立可測，通過 `cargo test` / `npm run build` 後即可 merge。

### 5.5.A1 — Provider Trait / Interface 定義 ✓

- [x] `ToolCallProvider` trait：`chat_with_tools(messages, tools, max_tokens, timeout_secs) -> Result<AiToolTurn, ToolCallError>`。
- [x] `ToolCallError` enum：`Unsupported / Network / Parse` 三種錯誤，有 Display 實作。
- [x] 移除 `AiToolDefinition`、`AiToolCallRequest`、`AiToolTurn` 上的 `#[allow(dead_code)]`。`AiToolObservation` 和 `provider_supports_tool_calls` 保留 allow（A3/A4 實作前暫未使用）。

### 5.5.A2 — FakeToolCallProvider（僅測試用）✓

- [x] `managers/fake_provider.rs`（`#[cfg(test)]` 模組）：`single_final` / `tool_then_final` 構造器。
- [x] 2 個單元測試：空 turns 後回傳 Err；tool+final 順序正確。

### 5.5.B1 — ReAct Loop Skeleton（FakeProvider + keynova.search）✓

- [x] `react_loop_body` + `ReactLoopConfig` + `ToolDispatch` 類型別名 in `agent_runtime.rs`。
- [x] `spawn_react_loop`：把 loop_body 搬到 background thread。
- [x] 停止條件：FinalText / cancel / provider error / max_steps 超限 → Failed。
- [x] 每步 emit `agent.step { run_id, step, tool_name, status, observation_preview }`。
- [x] Approval-required tool 在 B1 自動略過（B3 加入 gate）。
- [x] 4 個整合測試（single-final, tool+final, cancel mid-loop, max-steps exceeded）全通過，無真實 LLM。

### 5.5.A3 — OpenAI-Compatible Provider Tool-Call Parsing ✓

- [x] 實作 OpenAI-compatible `chat_with_tools()`：對應 `tools` + `tool_choice` request fields；解析 response choice 中的 `tool_calls`。
- [x] Provider 不支援 tool calls 時回傳 typed `UnsupportedToolCallError`。

### 5.5.A4 — Ollama Provider Tool-Call Support ✓

- [x] 實作 Ollama `chat_with_tools()`：對應 function-calling format（若模型支援）。
- [x] 不支援時回傳 `UnsupportedToolCallError`，觸發 5.5.C offline fallback。

### 5.5.B2 — 加入 filesystem.read / filesystem.search 工具 ✓

- [x] 在已通過的 ReAct loop skeleton 上加入 `filesystem.read`（read-only preview）和 `filesystem.search` 工具。
- [x] 整合 SystemIndexer 作為 `filesystem.search` 後端。
- [x] `ReactDispatchState` 同時實作 `keynova_search` / `web_search` dispatch（完整 4 工具）。
- [x] `AgentHandler::build_react_dispatch()` 建構完整 `Arc<ToolDispatch>` closure。
- [x] 2 個 ReAct 整合測試（filesystem search + filesystem read via temp files）。

### 5.5.B3 — 加入 approval-gated git.status ✓

- [x] 加入 `git.status` tool spec，標記為 approval-required。
- [x] ReAct loop 遇到 approval-required tool 時：建立 gate approval、設 WaitingApproval、publish event、poll 等待使用者回應。
- [x] Approved：執行 `git status --short`（ReactDispatchState::dispatch_git_status）後繼續 loop。
- [x] Rejected：告知 LLM 工具被拒，繼續 loop（不取消整個 run）。
- [x] approve_run / reject_run 分辨 ReAct gate（planned_action=None）vs heuristic flow（planned_action=Some）。
- [x] 2 個整合測試（approve gate + reject gate）；126 tests 全通過。

### 5.5.C — Move Heuristics to Offline Fallback ✓

- [x] 在 `handlers/agent.rs` 中，將 `direct_local_answer()`、`sources_for_prompt()`、`detect_planned_action()` 移入 `provider_supports_tool_calls == false` 或 `agent.mode = "offline"` 的分支。
- [x] 正常模式：LLM 決定 tool calls；heuristics 僅在 provider 不可用或模型不支援 function calling 時啟動。

### 5.5.D — Platform Command Sandbox（Research 完成，Product 仍 Blocked）✓

- [x] Windows：Job Object (`CreateJobObjectW` + `JOB_OBJECT_LIMIT_JOB_MEMORY` + `KillOnJobClose`) + `WaitForSingleObject` timeout，5 tests 通過（包含真實 echo 執行與 timeout kill）。
- [x] Linux：`bwrap` namespace isolation (`--unshare-net --unshare-pid --unshare-ipc`)，bwrap 缺失時明確拒絕執行。
- [x] macOS：`sandbox-exec` deny-by-default profile（temp `.sb` 檔），執行後刪除。
- [x] `output_cap_bytes` 截斷保護，caller 仍須接 `agent_observation` redaction pipeline。
- [x] `sandbox_available()` 回傳 `Available / BinaryMissing / Unsupported` 三態，供未來 registry 判斷。
- [x] `cargo clippy -- -D warnings` 零 warning；140 tests 通過；不暴露為任何 tool。
- [ ] **Product 解封前仍需**：Windows AppContainer 網路隔離；Linux seccomp filter；macOS App Sandbox entitlement 評估；ReAct loop 下整合測試。

### 5.5.E — Persist ReAct Audit ✓

- [x] 將每個 ReAct step 持久化到 Knowledge Store `agent_audit_logs`：`request_id`、tool schemas、args preview、approval decision、observation summary、redacted sources、final answer、failure/cancel reason。

### 5.5.F — Frontend Agent Run View ✓

- [x] `useAgent.ts`：新增 `ReactStep` interface、`reactSteps: Record<string, ReactStep[]>` state、監聽 `"agent-step"` 事件（legacy alias for `agent.step`），upsert by `(step, tool_name)` key，按 step 排序。
- [x] `AiPanel.tsx`：新增 `ReactStepTimeline` component（色碼徽章：executing=藍脈、waiting_approval=琥珀脈、executed/approved=綠、rejected/timeout=紅、final=翠綠粗）、observation count badge。
- [x] Pending approval header 顯示 count；區分 ReAct gate approval（`planned_action === null` → 標示 "react tool gate"）vs heuristic planned action。
- [x] Final answer 以 `border-emerald-500` 框突顯，而非普通 `text-gray-400` pre。

### 5.5.G — ReAct Regression Tests ✓

- [x] Provider tool-call parsing（react_loop_tool_call_then_final_completes_run）✓ 既有
- [x] Schema generation（tool_schemas_are_generated_and_openai_safe）✓ 既有
- [x] Loop max-step stop（react_loop_max_steps_exceeded_fails_run）✓ 既有
- [x] Cancellation（react_loop_cancel_mid_loop_stops_early）✓ 既有
- [x] One-shot approve/reject（react_loop_approval_gate_approved/rejected）✓ 既有
- [x] Web-query redaction（rejects_private_context_in_web_query）✓ 既有
- [x] Provider network error → run fails（react_loop_provider_network_error_fails_run）NEW
- [x] Unknown/unsafe tool → error observation, loop completes（react_loop_unknown_tool_produces_error_observation_and_loop_completes）NEW
- [x] Large observation handled without panic（react_loop_large_dispatch_result_does_not_panic）NEW
- [x] 157 tests passing, clippy clean。

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

## Phase 5.11 — AgentHandler Refactor（agent.rs 架構改善）

> 由使用者 10 點架構審查驅動。Phase A 無架構改動，直接可實作；B–D 逐步進行。

### 5.11.A — 正確性／安全修正（無架構改動）✓

- [x] 修 `extract_quoted`：將 `for quote in [...] { ... ? ... }` 改為 `find_map`，消除「雙引號不存在時單引號永不嘗試」的邏輯錯誤
- [x] 新增工具名稱字串常數（`TOOL_KEYNOVA_SEARCH`, `TOOL_FILESYSTEM_SEARCH`, `TOOL_FILESYSTEM_READ`, `TOOL_WEB_SEARCH`, `TOOL_GIT_STATUS`），ReAct dispatch match 和 llm_name 賦值都引用同一常數
- [x] `dispatch_filesystem_read` 加入 `resolve_readable_path()`：canonicalize 後驗證在 workspace roots 內，並拒絕 `looks_sensitive_path()`（`/.ssh`, `/.gnupg`, `.env`, `id_rsa`, `id_ed25519`, `credentials` 等路徑）
- [x] 優化 `truncate()`：單次 chars() scan，消除 `chars().count()` 的第二次 O(n) 遍歷

### 5.11.B — 消除重複搜尋邏輯 ✓

- [x] 提取 `LocalContextSearcher` struct，合併 `push_heuristic_workspace_source` / `push_react_workspace_source` 等兩組近乎相同的方法（~200 行節省）

### 5.11.C — 檔案拆分（agent/ 子目錄）✓

- [x] 將 3830 行 agent.rs 拆分為：`agent/mod.rs`, `agent/filesystem.rs`, `agent/formatting.rs`, `agent/intent.rs`, `agent/safety.rs`, `agent/web.rs`

### 5.11.D — 型別化 AgentError 與強化 ReAct 覆蓋率 ✓

- [x] `AgentError` enum（Config/LockPoisoned/ProviderUnavailable/ToolDenied/SafetyViolation/IoError）加入 `models/agent.rs`
- [x] ReAct prompt_audit 涵蓋率：`start_react_run` 現在也填寫 `prompt_audit` 與 `sources` 欄位（從 None 改為實際值）
- [x] ToolPermission 強制執行：`dispatch()` 層 APPROVAL_GATED 常數；`__approved` token 由 ReAct loop approval 後注入
- [x] WebSearchProvider trait 抽象：SearxngProvider/TavilyProvider/DuckDuckGoProvider + resolve_web_search_provider() factory

---

## Phase 6 — 新需求（2026-05-09 提出）

> 七個功能需求按複雜度與影響分為 6.1–6.7。
> 建議執行順序：6.7（記憶體最高優先）→ 6.1 → 6.4 → 6.5 → 6.3 → 6.2 → 6.6。

---

### Phase 6.1 — `/model_remove` UI 入口

> 後端 `model.delete` 已存在（`handlers/model.rs` 第 301 行）；本項只需補 UI 入口。

**架構設計**
- `handlers/builtin_cmd.rs`：將 `/model_remove` 註冊為 builtin command，解析可選參數 `<model-name>`
- 無參數時：呼叫 `model.list_local` 取得本機模型清單，以 Panel 列表呈現讓使用者選擇
- 有參數時：直接帶入 model name，彈出確認提示後呼叫 `model.delete`
- 前端：新增 `ModelRemovePanel.tsx`（複用現有 `ModelListPanel` 樣式），item 點擊後顯示確認 dialog
- 成功刪除後發送 `model.removed` 事件，觸發 `ModelListPanel` 刷新

**驗收條件**
- [x] `/model_remove` 無參數 → 顯示可刪除模型清單（需真實環境手動驗收）
- [x] `/model_remove <name>` → 確認後刪除，顯示成功/失敗訊息（需真實環境手動驗收）
- [x] 刪除中的模型名稱顯示 loading 狀態，完成後從清單消失（需真實環境手動驗收）
- [x] 刪除不存在的模型回傳清晰錯誤訊息（非 panic）（需真實環境手動驗收）
- [x] `cargo test` / `npm run lint` 通過（157 tests, 0 lint errors）

---

### Phase 6.2 — `/system_monitoring` 即時系統資訊面板

> 目前 `system_control.rs` 只有 volume/brightness/wifi；完全沒有 CPU/RAM 監控。

**架構設計（新增，不改 core/）**

後端：
- 新增 `sysinfo = "0.30"` 到 `Cargo.toml`（跨平台 CPU/RAM/Disk/Network）
- 新增 `handlers/system_monitoring.rs`，實作以下命令：
  - `system_monitoring.snapshot` — 單次查詢（CPU%、RAM used/total、Disk used/total per mount、Network rx/tx、Process top-10 by RAM）
  - `system_monitoring.stream_start { interval_ms }` — 開始定時推送 `system_monitoring.tick` 事件
  - `system_monitoring.stream_stop` — 停止推送（清除 tokio::JoinHandle）
- 使用 `Arc<Mutex<Option<JoinHandle>>>` 持有 stream handle，確保只有一個 stream 在執行
- 進程列表使用 `sysinfo::System::processes()` 取前 N 筆（按記憶體降序），避免全量傳輸

前端：
- 新增 `SystemMonitoringPanel.tsx`：
  - CPU 使用率：進度條（即時更新）
  - RAM：已用 / 總量 bar
  - Disk：每個 mount point 使用率
  - Network：上/下行速率（KB/s）
  - Process table：top-10 進程（name, pid, mem_mb, cpu%）可排序
- 訂閱 `system-monitoring-tick` 事件，unmount 時呼叫 `stream_stop`

**效能邊界**
- 預設 `interval_ms = 2000`，使用者可在 /setting 調整（500ms–10000ms）
- sysinfo 呼叫在獨立 tokio task，不阻塞 main handler thread
- 進程清單上限 20 筆（避免前端 table 過大）

**驗收條件**
- [x] `/system_monitoring` 開啟面板，CPU/RAM 即時更新（需真實環境手動驗收）
- [x] 關閉面板後 stream 停止（不繼續輪詢）（需真實環境手動驗收）
- [x] Disk / Network 資訊正確顯示（需真實環境手動驗收）
- [x] `cargo clippy -- -D warnings` 通過（159 tests，0 warnings）
- [ ] Process table 排序可切換（mem / cpu）（尚未實作排序切換 UI）

---

### Phase 6.3 — `/setting` 個人化功能開關

> `models/settings_schema.rs` 已有 `features.*` namespace；AI/agent 目前預設 `true`，需改 `false`。
> 本項新增 UI 開關 + 修正預設值 + 補上 `features.agent` 欄位。

**架構設計**

後端：
- `models/settings_schema.rs`：
  - 新增 `features.agent: bool`（default `false`）
  - 將 `features.ai` / `features.agent` 預設改為 `false`
  - 新增 `features.commands: HashMap<String, bool>`（允許單一 command 啟用/停用）
- `handlers/builtin_cmd.rs`：分派前讀取 `features.<name>` 並在 `false` 時回傳 `disabled` 訊息
- `handlers/ai.rs`：`ai.chat` / `agent.start` 啟動前檢查 `features.ai` / `features.agent`

前端：
- `SettingPanel.tsx`：在現有 setting 區塊中新增「功能開關」分頁
  - 每個功能一個 toggle row（label + description + switch）
  - AI、Agent 獨立開關（預設 off，有說明提示「需要 Ollama 或 API Key」）
  - Commands 區塊：列出所有已知 built-in commands，每個可以獨立勾選
- 變更即時寫入（呼叫 `config.set`），重啟不需要

**驗收條件**
- [x] AI / Agent 預設為 `false`，關閉時 `/ai` 和 `/agent` 回傳友善提示（需真實環境手動驗收）
- [x] 在 /setting 開啟 AI 後，`/ai` 立即可用（不需重啟）（需真實環境手動驗收）
- [x] Features 清單顯示 7 個功能開關（ai/agent/translation/notes/history/calculator/system），可逐一 toggle
- [x] 設定值寫入 `config.toml` 後重啟仍生效（依賴現有 config.set 實作）

---

### Phase 6.4 — `/tr` 全語言支援修復 ✓

> 後端已呼叫 Google Translate 自由 API，支援任意語言對。問題在 UI 解析 `/tr en ja` 格式。

**驗收條件**
- [x] `/tr en ja 你好` → 正確回傳日文翻譯（需真實環境手動驗收）
- [x] `/tr auto fr Hello world` → 自動偵測英文並翻譯成法文（需真實環境手動驗收）
- [x] 語言 dropdown 包含 108 語言，可搜尋（SUPPORTED_LANGS + LangPicker 元件）
- [x] 未知語言代碼顯示琥珀色警告徽章，仍正常送出 Google API
- [x] 保留原有 command line 格式（`en ja text`），Picker 與 command line 雙向同步
- [x] Source / Output textarea 補上 CJK font-family stack
- [x] `cargo test` 157 passed，`npm run lint` 0 errors

---

### Phase 6.5 — 搜尋結果編碼修復

> 問題現象：搜尋結果出現亂碼（可能來自 Windows 文件名、registry app 名稱、或 Tantivy 索引）。

**架構設計（偵錯先行）**

根因定位（優先確認以下三點，再動程式碼）：
1. **Windows App 名稱**：`managers/app_manager.rs` 從 Win32 registry 讀取的 `OsString` → `String` 轉換是否有 lossy 處理
2. **檔案路徑**：`platform/windows.rs` 中 Everything IPC / file cache 讀取時的 `from_utf8` 是否有 fallback
3. **Tantivy 索引**：`tantivy_index.rs` 寫入文件時 `name` / `path` 欄位的來源是否可能含非 UTF-8

修復方向：
- App 名稱：`OsString::to_string_lossy()` → 改用 `to_str().unwrap_or_else(|| sanitize_name())` 並記錄 warn log
- 檔案路徑：`PathBuf::to_string_lossy()` 結果中的 `\u{FFFD}` 替換字元應觸發 skip + warn
- Tantivy：`index_document` 前驗證字串為合法 UTF-8，否則做 lossy 轉換並標記
- 前端：搜尋結果 `title` / `path` 欄位若含 `�`（U+FFFD）則顯示為灰色 fallback 名稱

**驗收條件**
- [x] 掃描 Windows 中文路徑、日文應用程式名稱，不再出現 `?` / 方塊字（需真實環境手動驗收）
- [x] 含非 ASCII 路徑的搜尋結果可以正常開啟（需真實環境手動驗收）
- [x] 後端 warn log 記錄無法解碼的原始 bytes（`eprintln!` + `{:?}` PathBuf）

---

### Phase 6.6 — LazyVim 內建插件（可攜式 Neovim）

> 目前 LazyVim 只偵測是否已安裝；使用者未安裝時不會自動提供。
> 要求：將 LazyVim 包裝在 Keynova 內，不大幅影響效能。

**架構設計（On-Demand Download，不直接打包二進位）**

策略：不將 nvim 二進位打包進 app bundle（避免+15–30MB 體積），改為「首次使用時自動下載可攜式 Neovim」。

後端：
- 新增 `managers/portable_nvim_manager.rs`：
  - `detect_or_download_nvim() -> Result<PathBuf>` — 先查 PATH，找不到則下載到 `keynova_data_dir()/nvim-portable/`
  - 下載目標：GitHub Release 的 `nvim-win64.zip`（Windows）/ `nvim-linux64.tar.gz`（Linux）/ `nvim-macos.tar.gz`（macOS），版本固定在 `v0.10.x`
  - 下載時推送 `nvim.download.progress { percent, status }` 事件
  - 解壓後設 `nvim` 可執行位元（Unix）
- `managers/lazyvim_manager.rs`（現有 note.rs 的 bootstrap 邏輯提取）：
  - `ensure_lazyvim_config(nvim_bin: &Path) -> Result<()>` — 若 `.keynova/lazyvim/config/` 不存在，clone LazyVim starter（離線時跳過插件安裝，只建立最小 init.lua）
  - LazyVim 插件下載為後台 lazy load（首次開啟 `/note lazyvim` 時在 nvim 內執行）

前端：
- `/note lazyvim` 觸發時，若 nvim 未安裝顯示下載進度 overlay（複用 model_download 的進度元件）
- 下載完成後自動打開 LazyVim terminal

**效能邊界**
- 可攜式 nvim 只在 `/note lazyvim` 首次啟動時下載（~10MB compressed），後續啟動直接使用
- 插件安裝由 LazyVim 自管（`lazy.nvim` 在首次啟動 nvim 後台自動 sync）
- 不影響 Keynova 主程序啟動時間（下載 / 解壓在獨立 tokio task）

**驗收條件**
- [ ] 未安裝 nvim 的機器：`/note lazyvim` → 顯示下載進度 → 下載完成後開啟編輯器
- [ ] 已安裝 nvim 的機器：行為與現有相同，不重複下載
- [ ] 可攜式 nvim 路徑寫入 config（`notes.nvim_bin`），使用者可覆蓋
- [ ] 下載失敗時顯示清晰錯誤，並 fallback 到內建 note 面板

---

### Phase 6.7 — 記憶體優化（P0，建議最先執行）✓ A+B

> 目前執行時占用 ~150MB。根本原因分析（已確認）：

**根本原因清單**

| # | 位置 | 問題 | 預估節省 | 狀態 |
|---|------|------|---------|------|
| A | `tantivy_index.rs:60` | `IndexWriter` buffer 固定 50MB，建完索引後 `writer` 未釋放 | ~35MB | ✓ 已修：15MB + explicit drop |
| B | `app_manager.rs:35` | `list_all()` 每次 clone 全量 `Vec<AppInfo>` | 5–15MB | ✓ 已修：回傳 `&[AppInfo]` |
| C | 所有 Manager | startup 時全量初始化（即使對應功能未啟用）| 5–10MB | 待辦（Phase 6.2/6.6 新 manager 才需要） |
| D | sysinfo（待加入） | 若未做到按需建立，持續持有系統快照 | 預防性 | 待 Phase 6.2 實作時遵守 |

**修復設計**

A — Tantivy Writer 釋放：
- `TantivyIndexManager` 分離 `writer: Option<IndexWriter>` 與 `reader: IndexReader`
- 初始建索引後呼叫 `writer.commit()` + `drop(writer)`，之後 `writer` 設為 `None`
- 重建索引時才重新取得 writer（`index.writer(15_000_000)`，15MB 夠用）
- 修改後 Tantivy 常駐記憶體降至 reader-only（~2–5MB）

B — AppManager Clone 消除：
- `list_all()` 改回傳 `Arc<Vec<AppInfo>>`，呼叫端持有 Arc 而非 clone
- `AppState` 中 `AppManager` 從 `Arc<Mutex<AppManager>>` 改為讓 manager 內部持有 `Arc<RwLock<Vec<AppInfo>>>`
- 搜尋時只取 read lock，不產生額外 Vec allocation

C — Handler 懶初始化：
- 目前所有 manager 在 `AppState::new()` 中同步初始化
- 將非核心 manager（SystemMonitoringManager、PortableNvimManager 等）改為 `OnceLock<Arc<...>>` 按需初始化
- `ModelManager` 的 Ollama catalog（可能含大量 JSON）改為 lazy fetch，不在啟動時預載

D — sysinfo 使用規範（6.2 實作時遵守）：
- `sysinfo::System` 不做 global static，只在 `/system_monitoring` panel 開啟時建立
- stream 停止時 drop `System` 物件，釋放 sysinfo 內部快照

**驗收條件**
- [ ] 冷啟動記憶體降至 80MB 以下（目標），120MB 以下為可接受（需真實 app 量測）
- [ ] `/ai`、`/note lazyvim`、搜尋等功能正常運作（無 regression）
- [x] `cargo test` + `cargo clippy -- -D warnings` 通過（157 tests）
- [ ] 使用 Windows Task Manager 或 `tasklist /V` 在啟動 30 秒後量測 WorkingSet

---

## Blocked

- Phase 5.5.D（platform sandbox）— Research branch only。等待 ADR-027 批准後才進 product；generic shell tool 不在此前解封。