# tasks.md — Keynova 任務追蹤

> 完成任務請打 `[x]`，並在 `memory.md` 更新「上次完成」與「下一步」。
> 阻塞中的任務請移至「阻塞中」區塊並說明原因。

---

## 目前狀態

Keynova 已完成 Phase 0–3.8 的主要能力：Tauri/React/Rust 基礎架構、CommandRouter/EventBus、設定系統、App/File 搜尋、全域熱鍵、PTY 終端、滑鼠鍵盤控制、Flow Launcher 風格 UI、CLI control plane、AI、翻譯、工作區、筆記、計算機、剪貼簿歷史、系統控制，以及 `/model_download`、`/model_list` 的本地模型管理基礎。Phase 4 已完成第一輪核心收斂：ActionRef/ActionArena、UiSearchItem、action.run、schema-driven settings、Workspace Context v2、Knowledge Store DB worker、Agent mode 安全骨架、Automation/Plugin security foundation。

目前下一步是補齊 Phase 4 尚未完成的重型 runtime 項目：search chunk streaming / cancellation token、`lib.rs` app 模組拆分、Tantivy provider、Agent 真實 web/keynova tool 執行、WASM loader/hot reload。Phase 2 搜尋後端選擇、`search.backend` debug 顯示、`/rebuild_search_index` 本機索引重建入口、`[search]` 設定與 backend selection tests 已補齊。

---

## 已完成摘要

- [x] Phase 0–1：專案 scaffold、核心路由、EventBus、設定檔、App Launcher、全域熱鍵、PTY 終端、滑鼠鍵盤控制、透明啟動器 UI。
- [x] Phase 2.A/B：三模式輸入（Search / `>` Terminal / `/` Command）、TerminalPanel、BuiltinCommandRegistry、SettingPanel、PanelRegistry、設定熱重載。
- [x] Search 基礎：Everything IPC、AppCache fallback、D–Z 槽與 WSL home 掃描、搜尋結果模型。
- [x] BUG-3/5/6/7/8/9/10/11：設定儲存、包體積、ESC 行為、Tab 補全、runtime 設定生效、快捷鍵捕捉、終端 ESC 修復。
- [x] FEAT-1–5：`/setting key value`、`keynova start/down/reload/status`、指令參數提示、終端內容保留、設定表格鍵盤導航。
- [x] Phase 3.0–3.8：AI/translation/workspace/note/calculator/history/system namespace，Claude/Ollama/OpenAI-compatible provider 基礎，Workspace 3 槽，Note/History/System/Calculator panel。
- [x] BUG-12/13/14：panel ESC、`/command` 最頂項 ↑ 回輸入框、搜尋結果捲動時輸入框固定。
- [x] BUG-15：lazy-loaded panel 第一次開啟時視窗高度未重新量測，造成 UI 裁切、透明殘影與外層 scrollbar。
- [x] FEAT-6 主體：`/model_download`、`/model_list`、硬體偵測、Ollama 模型檢查/下載/刪除/啟用、模型下載進度事件、AI/Translation 分工具 provider/model 設定。
- [x] Phase 2 搜尋補齊：`search.backend` 前端 debug badge、`/rebuild_search_index` 背景重建本機 file cache、`[search]` 設定 schema、backend selection 單元測試；Tantivy provider 仍保留為重型後續項。

---

## 待手動驗收

- [ ] BUG-1：DevTools Performance 確認搜尋無多餘 IPC round-trip。
- [x] BUG-2：呼叫 `hotkey.register` IPC 收到明確 `NotImplemented` 錯誤。
- [x] BUG-4：DevTools Console 無 CSP violation；CSP 啟用後功能正常。
- [x] BUG-11：進入 `>` 終端後執行 `ipconfig`，實體 ESC 應立即退回搜尋模式且視窗不隱藏。
- [x] BUG-13：`/command` 最頂項按 ↑，焦點回到搜尋框。
- [x] BUG-14：搜尋結果超出視窗高度時，↓ 選到下方項目，輸入框仍可見。
- [x] BUG-15：第一次開啟 `/cal`、`/ai`、`/history` 等 lazy panel 時，視窗會在 panel 載入後自動 resize，且不再出現外層 scrollbar。
- [x] Phase 3.8：`npm run tauri build` 包體積確認（目標約 50MB）；Search / Terminal / Command / Setting / Hotkey / Mouse 全面手動回歸。
- [x] FEAT-6：`/model_download` 顯示 RAM/VRAM 與推薦模型。
- [x] FEAT-6：選擇 Claude API → 輸入 key → 儲存 → provider 切換。
- [x] FEAT-6：`/model_list` 顯示已下載模型 + 已設定 API，Enter 可切換使用中。
- [x] FEAT-6：`/model_list` 分別切換 AI Chat 與 Translation 模型，兩者互不覆蓋。
- [x] FEAT-6：手動貼上模型 URL 或名稱可下載並套用到目前工具。

---

## 進行中

### FEAT-7 — `/tr` Google Translate only

目標：零設定即可使用翻譯；`/tr` 固定使用 Google Translate 免費端點，不再提供模型 provider。

保留 provider：

| 順序 | Provider | 條件 | 備註 |
| --- | --- | --- | --- |
| 1 | Google Translate 免費端點 | 無需 key | 唯一翻譯 provider；需處理 rate limit |

任務：

- [x] 拔除翻譯模型 provider：`/tr` 不再使用 Ollama / Claude / OpenAI-compatible。
- [x] `/model_download`、`/model_list` 移除 Translation 工具切換，只保留 AI Chat 模型管理。
- [x] 實作 Google 免費端點：`translate.googleapis.com/translate_a/single?client=gtx`，解析巢狀陣列回應。
- [x] 實作 rate limit / 網路錯誤的明確錯誤訊息，讓前端可直接顯示。
- [x] `default_config.toml`：移除 `translation.model` / `translation.google_api_key`，保留 `provider = "google_free"`。
- [x] 驗收：零設定 `/tr en zh-TW hello` → `你好`；模型清單與下載面板不再顯示 Translation。

---

## Phase 2 補齊

- [ ] Tantivy 離線索引（Everything 不可用時的完整檔案搜尋後端）。
- [x] `search.backend` 前端顯示（搜尋框角落顯示目前使用後端）。
- [x] `cmd_rebuild_search_index`：手動重建目前本機搜尋 file cache；Tantivy provider 接入後沿用同一入口。
- [x] `default_config.toml` 加入 `[search]` 區塊（索引路徑、排除目錄、backend 偏好）。
- [x] 單元測試：`test_backend_selection_*`。

---

## 架構可行性確認

架構書方向可行，原因是目前 repo 已經具備必要骨架：

- `CommandRouter` 已用 `namespace.command` 路由，適合被 Action System 包裝。
- `BuiltinCommandRegistry` 已集中 `/command` metadata，可直接轉成第一批 built-in actions。
- `EventBus` 已能廣播後端事件，可演進成 Automation trigger 來源。
- `PanelRegistry` 已集中 panel 對應，適合由 `ActionResult::Panel` 統一驅動。
- `WorkspaceManager` 已有 3 槽位與 JSON persist，可漸進升級成 Workspace Context v2。
- `SearchRegistry` 目前幾乎是空殼，正好可承接 Federated Search provider 化。
- `HistoryManager`、`NoteManager`、`WorkspaceManager` 目前偏 JSON 分散持久化，後續導入 Knowledge Store 有明確收益。
- `lib.rs` 已承載 state 初始化、handler 註冊、watcher、tray、shortcut、control plane，拆分成 `app/*` 模組是必要且低風險的維護性重構。

主要風險：

- Action System 若一次替換所有 handler，風險過大；應先包裝現有 command/panel/result，而不是重寫路由。
- Automation Engine 依賴穩定的 ActionResult、EventBus 命名與 action history，不能早於 Action System。
- `/ai` Agent 不能只是把 LLM 接上 shell；必須先有 Action System、Workspace Context、Knowledge Store、approval gate 與 audit log，才允許它執行本機動作。
- Plugin Runtime 需要 permission model、settings schema、action/search provider 邊界，應最後做。
- Knowledge Store 若太早大搬遷會影響 notes/history/workspace；應先新增資料層與 migration，再逐步切換。

---

## Phase 4 核心效能與安全約束

### IPC Boundary

- 前端不得接收完整 Action payload。
- 搜尋結果只傳 ViewModel + `ActionRef`。
- Action payload 保存在 Rust `ActionArena`。
- secondary actions lazy resolve。
- search results chunked streaming。
- IPC payload size / serialization latency 納入 observability。

### Database Runtime

- `rusqlite` 不得直接在 Tauri command hot path 執行。
- Knowledge Store 使用 dedicated DB worker thread。
- 所有 writes 透過 bounded channel。
- high-frequency logs 支援 batch / drop policy。
- search ranking 使用 memory cache，不在每次 keystroke 查 DB。
- DB latency / queue length / dropped logs 納入 observability。

### Plugin Isolation

- 禁止 WebView eval 作為 plugin runtime。
- Plugin Runtime 採 WASM-first。
- 所有能力透過 Host Functions。
- 預設 deny filesystem / network / shell / clipboard / AI key / system control。
- 每次 plugin execution 有 memory / time / host-call / output limits。
- Plugin audit log 為必備功能，不是後續優化。

---

## 優化後 Phase 4 路線

### Phase 4.0 — Stability & Observability

目標：讓 Keynova 能長時間穩定常駐，並讓後續重構有回歸基準。

- [x] 新增 `docs/regression.md`：Search / Terminal / Command / Setting / Hotkey / Mouse / AI / Translation / Model / Workspace 手動驗收。
- [x] 統一 handler 錯誤格式：`{ code, message, details? }`，前端保留向後相容。
- [x] EventBus 命名整理成 `domain.entity.action`，先建立文件與 adapter，不一次改完所有事件。
- [x] 記錄 IPC command latency、search latency、action execution latency。
- [x] 記錄 idle memory / CPU baseline、production build size baseline。
- [x] 將 Windows PowerShell clipboard polling 改為原生 listener 或低成本 watcher。
- [ ] 拆分 `lib.rs` 到 `src-tauri/src/app/*`：`state.rs`、`bootstrap.rs`、`shortcuts.rs`、`tray.rs`、`window.rs`、`watchers.rs`、`control_server.rs`。

### Phase 4.1 — Action IPC Boundary

目標：把執行權與 action lifecycle 留在 Rust，前端只接收顯示用 ViewModel 與引用，避免搜尋熱路徑傳送肥大的 JSON。

- [x] Action payload 不直接跨 IPC 傳輸。
- [x] 新增 `ActionRef`，支援 stable id 與 ephemeral session ref。
- [x] 新增 `ActionArena`：Rust 端保存 search/session-scoped actions。
- [x] `SearchResult` 改為只回傳 `UiSearchItem`：`item_ref`、`title`、`subtitle`、`source`、`score`、`icon_key`、`primary_action`、`primary_action_label`、`secondary_action_count`。
- [x] secondary actions lazy resolve：展開時才呼叫 `action.list_secondary`。
- [x] `action.run` 只接收 `ActionRef` + input。
- [x] `ActionRef` 驗證 `session_id` / `generation` / TTL，避免 stale action 執行。
- [ ] 搜尋結果支援 chunked streaming；首批結果目標 < 50ms。
- [ ] 搜尋首批回傳 10–20 筆，即時顯示上限 50 筆；更多結果分頁或串流。
- [ ] icons / preview / metadata lazy load。
- [x] 每次 IPC payload size 與 serialization latency 加入 debug log。

### Phase 4.2 — Unified Action System（漸進包裝）

目標：先讓既有 command/panel/search result 產生 action，再逐步替換內部模型。

- [x] 新增 `models/action.rs`：`Action`、`ActionRef`、`ActionResult`。
- [x] 新增 `core/action_registry.rs`。
- [x] `BuiltinCommandRegistry` metadata 可轉成 built-in actions。
- [x] `CommandUiType` 漸進映射到 `ActionResult::Inline` / `ActionResult::Panel`。
- [x] 新增 `action.run` handler，先委派到既有 `CommandRouter`。
- [x] SearchResult 不保存完整 `Action`；僅保存 `ActionRef` 與顯示必要欄位。
- [x] CommandPalette 支援 secondary actions。
- [x] 新增 action history 與 recent/frequent ranking。

### Phase 4.3 — Workspace Context v2 + Schema-driven Settings

目標：讓工作區從 UI 狀態槽位升級成完整上下文容器，同時用 schema 降低設定 UI 與 `/setting` 維護成本。

- [x] `WorkspaceState` 加入 `name`、`project_root`、`recent_actions`。
- [x] workspace persist 加入 version，提供舊 JSON migration。
- [ ] terminal session 綁定 workspace。
- [ ] note 綁定 workspace。
- [ ] AI conversation 綁定 workspace。
- [ ] clipboard/history 搜尋可依 current workspace 加權。
- [x] 支援 workspace restore：query、mode、panel、recent files、terminal sessions。
- [x] 新增 `SettingSchema` / `SettingValueType`。
- [x] `ConfigManager` 保留 flat key API，但 schema 負責型別、預設值、選項、敏感欄位。
- [x] SettingPanel 由 schema 生成欄位。
- [x] `/setting` arg suggestions 由 schema 生成。
- [x] 敏感欄位（API key）遮蔽顯示並避免寫入 log。

### Phase 4.4 — Federated Search with Cancellation/Chunks

目標：搜尋所有可操作的東西，而不只 app/file；搜尋熱路徑必須可取消、可分批、可控制 payload。

- [x] 將 `core/search_registry.rs` 實作成 provider registry。
- [x] 新增 `SearchProvider` trait：`id()`、`search(query, context)`。
- [x] 擴充 `UiSearchItem/SearchResult`：`source`、`subtitle`、`score`、`primary_action`、`primary_action_label`、`secondary_action_count`。
- [x] Provider：App、File、Command、Note、History、Model。
- [x] Ranking merge：來源權重、workspace boost、recent/frequent action boost。
- [x] 前端顯示 provider badge 與 secondary actions。
- [x] 每次 query 產生新 generation；舊 provider result 必須可丟棄。
- [ ] Provider 支援 cancellation token / generation check。
- [ ] Progressive results 透過 Tauri channel chunk 傳送。
- [ ] Plugin/provider timeout 後直接跳過，不阻塞主搜尋。
- [ ] Tantivy 離線索引整合到 File provider。
- [x] `cmd_rebuild_search_index` 走 Action System。

### Phase 4.5 — Knowledge Store with DB Worker

目標：集中 notes、history、workspace、action logs、search metadata，但 SQLite 不進入 Tauri command / search hot path。

- [x] 評估並優先採用 `rusqlite`。
- [x] 新增 `KnowledgeStoreHandle`，對外只暴露 async message API。
- [x] 新增 dedicated DB worker thread，rusqlite connection 只存在於該 thread。
- [x] 新增 `DbRequest` enum，支援 fire-and-forget write 與 oneshot response。
- [x] 所有 writes 透過 bounded mpsc channel。
- [x] 需要回傳值的 reads/writes 使用 oneshot response + timeout。
- [x] 資料表：`actions`、`action_logs`、`clipboard_items`、`notes_index`、`ai_conversations`、`workspace_contexts`、`search_index_metadata`、`workflow_definitions`。
- [x] 先雙寫 action logs / clipboard metadata，再逐步切換讀取來源。
- [x] action_logs / clipboard metadata 支援 batch insert。
- [x] high-frequency logs 定義 drop policy，channel 滿時不阻塞 UI。
- [ ] search ranking 所需 recent/frequent stats 啟動時預載入 memory cache。
- [x] 搜尋 hot path 不直接查 SQLite。
- [x] SQLite 啟用 WAL / `busy_timeout` / `foreign_keys`。
- [x] app shutdown 時 flush pending writes。
- [x] DB latency / queue length / dropped logs 加入 observability。
- [ ] 補 migration 與備份策略。

### Phase 4.5A — `/ai` Agent Runtime（從 LLM Chat 升級為工作代理）

目標：讓 `/ai` 從單次 LLM 對話升級為可理解工作區、規劃步驟、呼叫受控 actions、保留工作記憶、並在高風險操作前要求確認的本機 AI agent。

原則：

- `/ai` 預設仍可當 Chat 使用；Agent mode 需明確進入或由 UI 明確切換。
- Agent 不直接呼叫 shell / filesystem / clipboard / system control；所有能力都走 Action System 或受控 Host Tool。
- 高風險 action 必須先顯示 plan + diff/impact，等待使用者確認後才執行。
- Agent 可讀 workspace context 與 knowledge memory，但 API key / secrets 預設遮蔽。
- 專案架構內容預設視為 `private_architecture`：可被本地索引用於 Keynova UI 搜尋，但不得注入 `/ai` prompt、不得送到外部 LLM、不得出現在 web search query rewrite。
- 所有 agent run、tool call、approval、error 都必須寫入 audit log，後續可搜尋與回放。

依賴順序：

- 依賴 Phase 4.1 `ActionRef` / `action.run`，避免前端或 LLM 持有完整 action payload。
- 依賴 Phase 4.2 Unified Action System，讓 Agent 只能透過統一 action 執行能力。
- 依賴 Phase 4.3 Workspace Context v2，讓 Agent 知道目前 project/root/query/panel/recent actions。
- 依賴 Phase 4.5 Knowledge Store，保存 agent memory、run log、tool audit 與 recent/frequent context。
- 依賴 Phase 4.7 的 permission model 思路，尤其 shell/filesystem/network/clipboard/system control 的 deny-by-default。

任務：

- [x] 新增 `docs/ai_agent.md`：Agent 目標、非目標、能力邊界、approval policy、風險等級。
- [x] 新增 `docs/ai_context_privacy.md`：定義 context visibility、資料分類、prompt allowlist、redaction、audit policy。
- [x] 新增 Agent 資料模型：`AgentRun`、`AgentStep`、`AgentToolCall`、`AgentApproval`、`AgentMemoryRef`、`AgentRunStatus`。
- [x] 新增 context visibility enum：`public_context`、`user_private`、`private_architecture`、`secret`，預設 unknown source 走 deny。
- [x] 新增 context allowlist：只有 `public_context` 與使用者明確選取的 `user_private` 可進 LLM prompt。
- [x] 新增 architecture denylist：`CLAUDE.md`、`tasks.md`、`memory.md`、`decisions.md`、`skill.md`、`docs/*architecture*`、`files/*架構*`、內部 cache/schema/debug 文件預設標為 `private_architecture`。
- [x] `/ai` UI 分成 Chat mode / Agent mode；Agent mode 顯示 plan、目前 step、tool calls、approval prompt、run log。
- [x] 新增 `agent.start` / `agent.cancel` / `agent.resume` / `agent.approve` / `agent.reject` handlers。
- [x] 新增 `AgentRuntime` loop：understand → plan → request approval if needed → run action/tool → observe result → continue/stop。
- [x] 新增 `AgentToolRegistry`：所有 agent tools 必須宣告 risk level、visibility rules、timeout、result limit、audit fields。
- [ ] 新增 `web.search` tool：透過設定 provider 執行基本網路搜尋，第一階段只回 title/url/snippet，不抓完整頁面。
- [ ] `web.search` provider 介面支援 SearXNG / Brave Search / Tavily 之一或多個；API key 設為 sensitive setting，不寫入 log。
- [ ] `web.search` query rewrite 不得包含 `private_architecture` / `secret` 內容，只能使用使用者問題與 allowlisted context。
- [ ] 新增 `keynova.search` tool：搜尋 Keynova 本地內容與 runtime cache，但回傳前套用 visibility filter。
- [ ] `keynova.search` 第一階段來源：notes metadata/content、clipboard/history 摘要、workspace summary、command/action metadata、model catalog、非敏感設定 schema。
- [ ] `keynova.search` 可本地索引 project docs/cache 以支援 UI 搜尋，但 `/ai` 只接收 allowlisted snippets；`private_architecture` 結果只可顯示「已隱藏架構內容」摘要。
- [x] 新增 `GroundingSource` ViewModel：`source_id`、`source_type`、`title`、`snippet`、`uri?`、`score`、`visibility`、`redacted_reason?`。
- [x] Agent tools 第一階段只允許低風險 read-only：`web.search`、`keynova.search`、list commands/actions、read workspace summary、read notes/history summary、ask model。
- [ ] Agent tools 第二階段允許中風險 actions：open panel、create note draft、update setting draft、run safe built-in command；執行前需確認。
- [ ] Agent tools 第三階段才評估高風險 actions：terminal command、file write、system control、model download/delete；必須逐次 approval。
- [x] Agent plan 必須可中止；取消後停止後續 tool calls 並寫入 run status。
- [x] Agent run 支援 streaming status event：`agent.run.started`、`agent.step.started`、`agent.tool.requested`、`agent.approval.required`、`agent.run.completed`、`agent.run.failed`。
- [ ] Agent 記憶分層：session memory、workspace memory、long-term memory；long-term 寫入需使用者確認或明確設定。
- [ ] Agent prompt 必須注入 workspace context，但限制 token budget，避免把所有 notes/history 全量塞入 prompt。
- [ ] Agent prompt builder 必須先做 context budget + visibility filtering + secret redaction；任何 filtered source 只記錄 audit，不傳給 model。
- [x] Agent output 必須區分「建議」與「已執行」；禁止模糊描述造成使用者以為已完成。
- [ ] Agent audit log 寫入 Knowledge Store：run id、model/provider、tool name、action ref、approval decision、duration、error。
- [ ] 新增 agent regression：取消、approval 拒絕、tool error、stale ActionRef、provider timeout、secret redaction、architecture context denied、web query redaction。
- [ ] 驗收：輸入「幫我整理目前工作區並列出下一步」時，Agent 只讀 context 並產生 plan，不執行破壞性動作。
- [ ] 驗收：輸入「幫我查一下最新 Tauri plugin 寫法」時，Agent 可呼叫 `web.search` 並列出來源。
- [ ] 驗收：輸入「幫我找 Keynova 裡跟模型下載有關的紀錄」時，Agent 可呼叫 `keynova.search`，但不暴露 private architecture。
- [ ] 驗收：輸入「把專案架構摘要給模型參考」時，Agent 應拒絕注入 `private_architecture` context，並說明需要改 visibility policy 才能使用。
- [ ] 驗收：輸入「幫我建立一則 note 草稿」時，Agent 顯示草稿與 action plan，確認後才寫入 note。
- [ ] 驗收：輸入「幫我跑測試」時，Agent 顯示將執行的命令與風險，確認前不啟動 terminal/process。

### Phase 4.6 — Automation Engine

目標：讓重複工作流程可被自動化，先從 TOML/YAML workflow 做起。

- [x] Workflow definition 格式。
- [x] Workflow parser。
- [x] Trigger registry：command、hotkey、clipboard_changed、file_changed、schedule、terminal_output_match。
- [ ] Action chain executor。
- [x] workflow execution log。
- [x] workflow error display。
- [x] 暫不做 GUI workflow builder，等核心穩定後再評估。

### Phase 4.7 — Plugin Security Model

目標：先定義 plugin threat model、能力邊界與資源限制，再開始 runtime 實作。

- [x] 新增 `docs/plugin_security.md`：threat model、trust boundary、permission model。
- [x] 禁止 WebView eval / unrestricted JS plugin。
- [x] Plugin Runtime 採 WASM-first，JS runtime 延後。
- [x] 評估 wasmtime Component Model 作為 plugin ABI。
- [x] 所有 plugin 能力必須透過 Host Functions。
- [x] 預設不提供 shell / filesystem / network / clipboard / AI key / system control。
- [x] 新增 resource limits：memory、execution time、host call count、output size、search result count。
- [x] Plugin Search Provider 必須有 timeout / cancellation / result limit。
- [x] Plugin audit log 記錄 host calls、permission denied、network target、file scope。
- [x] Plugin crash 不得影響主程序。

### Phase 4.8 — WASM Plugin Runtime

目標：讓外部腳本安全擴充 Keynova；此階段必須晚於 Action/Search/Settings/Automation。

- [x] Plugin manifest。
- [x] Plugin loader。
- [ ] WASM runtime proof-of-concept。
- [ ] Host Functions API：log、get_setting、search_workspace、read_clipboard、http_fetch、run_action。
- [x] Manifest permission parser 與 permission check。
- [ ] Plugin action registration。
- [ ] Plugin search provider registration。
- [ ] Plugin settings schema。
- [ ] Plugin audit log 寫入 Knowledge Store。
- [ ] Hot reload。
- [x] JS plugin 僅作為後續便利層評估，若使用 `deno_core`，必須維持 capability-based ops 與相同 resource/audit 約束。

---

## 高價值後續功能

- [ ] Smart Clipboard：辨識 `code/url/path/email/json/error log/command/markdown/image`，並提供對應 actions。
- [ ] Dev Workflow Pack：`git status`、`git branch`、`cargo test`、`npm run build`、`open project`、`search symbol`、`explain compiler error`、`create commit message`。
- [ ] Command Chaining：`/search error | ai explain | note append`。
- [ ] Universal Command Bar：明確前綴優先，無前綴時搜尋 app/file/note/history/command/action，再由 AI 作為 fallback。

---

## 持續維護規則

- [ ] 快捷鍵文件同步：每次新增、修改或移除快捷鍵後，同步更新 `README.md` 的「快捷鍵速查」區塊。
- [ ] 新增 panel 時必須同步檢查 ESC 行為、鍵盤導航、PanelRegistry、手動驗收清單。
- [ ] 新增 provider/action/search source 時必須補 metadata、錯誤訊息、手動驗收步驟。

---

## 阻塞中

- 無
