# Keynova Tasks

> 完成任何批次後須更新 `docs/memory.md` 與對應 ADR。

---

## 最高原則：功能不變（Feature Freeze During Refactor）

**所有 TD 階段執行期間，下列功能的行為必須完整保留：**

| 功能 | 主要入口 | 驗收方式 |
|------|----------|----------|
| 搜尋（app/file/note/history/command/model） | `CommandPalette` → `search.query` | 手動：中文/日文路徑、app 啟動、note 開啟 |
| 搜尋串流（chunked progressive） | `search-results-chunk` event | 手動：快速輸入不閃爍，慢速 file 有進度 |
| 終端（PTY / workspace） | `/terminal` → `terminal.open` | 手動：開終端、輸入命令、關閉 |
| LazyVim（`/note lazyvim`） | `NoteCommand` → `TerminalLaunchSpec` | 手動：nvim 開啟、Escape 正常、退出快捷鍵 |
| AI Chat（`/ai`） | `AiHandler::chat_async` | 手動：Ollama/OpenAI provider 切換與回應 |
| Agent ReAct loop（`/agent`） | `AgentHandler::start_react_run` | 手動：tool call、approval、cancel |
| 翻譯（`/tr`） | `TranslationHandler` | 手動：108 語言 picker、auto 偵測 |
| 設定（`/setting`） | `SettingPanel` | 手動：熱鍵、provider、feature 開關 |
| 系統監控（`/system_monitoring`） | `SystemMonitoringPanel` | 手動：CPU/RAM/Disk/Network 更新、stream 停止 |
| 面板路由（PanelRegistry） | `CommandUiType::Panel(name)` | Cargo + TSC：registry key 對齊 |

**驗收底線（每次 merge 前必須通過）：**
```
npm run lint && npm run build
cargo test
cargo clippy -- -D warnings
```

---

## 已完成（Completed）

- **Phase 0–6.7** + **DOCS + Plan A**：Tauri/React/Rust scaffold；CommandRouter/EventBus；launcher UI；app/file/note/history/command/model 搜尋；terminal/PTY；settings/hotkeys；mouse/system control；AI chat；Agent ReAct（tool schema、approval gate、audit、web search、sandbox infra）；翻譯 108 語言；系統監控；LazyVim portable；記憶體優化；docs/ 完整重組（28 ADR）；CLAUDE.md 化簡；根目錄清理。
- **Test count**：157+ Rust unit/integration tests。
- **詳細歷史**：見 `docs/memory.md` 歷史摘要與 Session 交接紀錄。

---

## 待辦（Pending）

### 5.5.H — ReAct 手動驗收

- [ ] JSON file find → `filesystem.search` 回傳結果，LLM 提取路徑後 `filesystem.read`。
- [ ] 檔案 read-only preview：LLM 讀取小型文字檔並摘要內容。
- [ ] Web query：SearXNG / Tavily / DuckDuckGo fallback 的完整 round-trip。
- [ ] Denied shell request：`__approved` token 未注入 → `ToolDenied` observation。
- [ ] git.status approval gate：UI 顯示 pending → 使用者 approve → 執行 → loop 繼續。
- [ ] Project search via Everything / Tantivy / SystemIndexer 路徑正確。
- [ ] Stale index warning 顯示在 observation。
- [ ] Rejected approval self-correction：LLM 在被拒後換工具繼續。
- [ ] Final answer grounded：引用搜尋結果而非幻想。

### 5.6 — Agent Quality & Regression Tests

- [ ] No-action run：Agent 說明 Keynova 下一步能做什麼（不亂 match panel/action）。
- [ ] 高風險 action 維持在 explicit approval 後方（不能繞過）。
- [ ] Regression test suite：cancellation/reject lifecycle、one-shot approval、tool error surfacing、prompt budget truncation、private architecture denial、secret redaction、web-query redaction。
- [ ] External agent integration contract：最小 provider 介面，允許 plan + tool requests，不繞過 approval/audit。
- [ ] Manual validation script：local search、feature search、private architecture denial、note draft approval、terminal approval、rejected approval、disabled web search、action payload handoff。

### 6.6 — 後續補強

- [ ] 可攜式 nvim 路徑寫入 config（`notes.nvim_bin`），使用者可在 `/setting` 覆蓋。

### 6.7.C+D — 記憶體優化（後半）

- [ ] **C — Handler 懶初始化**：`PortableNvimManager`、`SystemMonitoringHandler` 的 `System` 物件改為 `OnceLock`；`ModelManager` Ollama catalog 改為 lazy fetch。
- [ ] **D — 量測**：冷啟動 WorkingSet 降至 120MB 以下（Task Manager 量測）。

### 5.9 — Plugin & WASM Runtime

- [ ] WASM runtime proof-of-concept。
- [ ] Host Functions API：`log`、`get_setting`、`search_workspace`、`read_clipboard`、`http_fetch`、`run_action`。
- [ ] Plugin action/search provider 註冊、settings schema、audit log、hot reload。

### 5.10 — Future Features

- [ ] Smart Clipboard（classify code/url/path/email/json/error log/command/markdown/image）。
- [ ] Dev Workflow Pack（git status、cargo test、npm build、open project、search symbol、explain compiler error）。
- [ ] Command chaining：`/search error | ai explain | note append`。
- [ ] Universal Command Bar（跨來源 AI fallback）。

---

## TD.1 — 前端邊界重構（功能不變，只切邊界）

> 來源：技術債報告 §8.1 + §4 P0 + §7。原則：不改任何行為，每個子項可獨立 merge。

### TD.1.A — 統一前端 IPC Client

- [ ] 新建 `src/ipc/client.ts`：`dispatch<T>(route, payload?)` / `dispatchVoid(route, payload?)`，內部呼叫 `invoke("cmd_dispatch", { route, payload: payload ?? null })`，統一錯誤格式 `{ code, message }`。
- [ ] `useIPC.ts` 與 `useCommands.ts` 改為 re-export `dispatch`，不直接 import `invoke`。
- [ ] 現有元件（`CommandPalette.tsx`、`AiPanel.tsx`、`SystemMonitoringPanel.tsx`、`NvimDownloadPanel.tsx` 等）改為 import `dispatch`。
- [ ] 驗收：`grep -r "from \"@tauri-apps/api/core\"" src/` 只剩 `src/ipc/client.ts`；`npm run lint && npm run build` 通過。

### TD.1.B — ESLint 限制直接 invoke

- [ ] `.eslintrc.cjs` 加入 `no-restricted-imports` 規則，禁止直接 import `invoke` from `@tauri-apps/api/core`，強制使用 `src/ipc/client.ts`。
- [ ] 驗收：`npm run lint` 通過（TD.1.A 完成後）。

### TD.1.C — CommandPalette 搜尋邏輯提取為純函式

- [ ] 建立 `src/features/launcher/domain/searchRanking.ts`：`sortSearchResults`、`applySourceQuotas`、`mergeSearchResults`。
- [ ] 建立 `src/features/launcher/domain/inputMode.ts`：`parseInputMode(query) → command | search | empty`。
- [ ] `CommandPalette.tsx` 改為 import 純函式，不重複邏輯。
- [ ] 驗收：純函式無任何 Tauri API import；`npm run build` 通過。

### TD.1.D — 補 Vitest 單元測試

- [ ] 安裝 Vitest，`package.json` 加 `"test": "vitest run"`。
- [ ] `searchRanking.test.ts`：sort（app > command > note > file）、applySourceQuotas（各來源上限）、mergeSearchResults（replace:true/false）。
- [ ] `inputMode.test.ts`：`/` 開頭→command；空字串→empty；一般文字→search。
- [ ] 驗收：`npm run test` 通過。

### TD.1.E — 拆分 CommandPalette hooks

每個 hook 獨立 PR，完成後立即 commit：

- [ ] `useLauncherSearch.ts`：search.query、request-id、chunk listener、stale-request 忽略、loading state、cancel on unmount。
- [ ] `useLauncherWindowSize.ts`：Tauri window innerSize + resize listener。
- [ ] `useSearchMetadata.ts`：search.metadata 惰性載入（hover/select 觸發，有 cache）。
- [ ] `useSearchIcons.ts`：search.icon 惰性載入，icon_key → data URI cache。
- [ ] `useLauncherKeyboard.ts`：Up/Down/Enter/Escape/Tab 導航狀態機，無 IPC。

---

## TD.2 — 後端 Composition Root 重構（功能不變）

> 來源：技術債報告 §4 P0 + §5.1 + §8.2。原則：不改任何 handler 行為，只改組裝方式。

### TD.2.A — 建立 app/container.rs

- [ ] `AppContainer::build()` 集中建立所有 manager（event_bus、action_arena、knowledge_store、config、note/history/search/workspace/model/ai/agent/translation/terminal manager）。
- [ ] `AppState::new()` 改為：build container → register_all_handlers → return。
- [ ] 驗收：`AppState::new()` 主體 < 30 行；`cargo test` 通過。

### TD.2.B — 建立 FeatureModule trait

- [ ] `src-tauri/src/app/modules.rs`：`trait FeatureModule { fn register_handlers(&self, router: &mut CommandRouter, container: &AppContainer); }`。
- [ ] 先包裝 SearchModule、AgentModule、TerminalModule；`register_all_handlers()` 接受 `&[Box<dyn FeatureModule>]`。
- [ ] 驗收：新增 feature 只需新建 module struct + 在 module list 加一行；`cargo test` 通過。

### TD.2.C — Builtin Command Registry 整理

- [ ] 各 BuiltinCommand 透過 `FeatureModule::register_handlers` 自行註冊，移除 `AppState::new()` 中的手動 `reg.register(...)` 列舉。
- [ ] 驗收：`AppState::new()` 無 `reg.register(...)` 呼叫；`cargo test` 通過。

---

## TD.3 — Typed IPC & DTO（功能不變）

> 來源：技術債報告 §4 P0 + §5.2 + §8.3。原則：保留相容層，新增 typed DTO 作並行路徑。

### TD.3.A — 後端 ipc/dto/

- [ ] `src-tauri/src/ipc/dto/`：`SearchQueryRequest`、`AgentStartRequest`、`SettingSetRequest`（Deserialize）。
- [ ] 對應 handler 加 typed parse helper，解析失敗回傳 `{ "error": "invalid_payload", "field": "..." }`。
- [ ] 先轉換：`search.query`、`agent.start`、`setting.set`；舊 `Value` 路徑保留。
- [ ] 驗收：`cargo test` 通過；parse error 有一致 error code。

### TD.3.B — 前端 ipc/routes.ts

- [ ] `src/ipc/routes.ts`：`ROUTES = { SEARCH_QUERY, AGENT_START, SETTING_SET, ... } as const`。
- [ ] `src/ipc/dto/`：search/agent/setting TS 型別，與後端 DTO 對齊。
- [ ] 驗收：route 字串不再在元件層出現；`npm run build` 通過。

### TD.3.C — ConfigManager Typed Config Snapshot

- [ ] `src-tauri/src/core/typed_config.rs`：`AppConfig { features, search, notes, ai, ... }` typed struct（Deserialize, Default）。
- [ ] `ConfigManager::typed_snapshot() -> Arc<AppConfig>`（read-only snapshot，不移除舊 HashMap）。
- [ ] 各 handler 改用 `typed_snapshot()` 讀取 bool/int 設定，不再用 `.get("...").parse::<bool>()`。
- [ ] 驗收：`cargo test` 通過；bool/int 設定不再靠字串猜測。

---

## TD.4 — Actor-Like Services（功能不變）

> 來源：技術債報告 §5.1 + §6.2 + §6.3 + §8.4。原則：Service 對外 API 不變，只換內部實作。

### TD.4.A — Agent Approval 改成 Async Channel

- [ ] `AgentRunHandle { cancel_tx: watch::Sender<bool>, approval_tx: mpsc::Sender<ApprovalDecision> }`。
- [ ] `spawn_react_loop` 改為 tokio async task，使用 cancel token 與 approval channel。
- [ ] 新增測試：approval timeout → run 取消；cancel signal → loop 停止。
- [ ] 驗收：`cargo test` 通過；手動：approve/reject/cancel 正常；audit log 仍寫入。

### TD.4.B — SearchService 獨立 Worker

- [ ] `SearchService` 持有 `mpsc::Sender<SearchRequest>`（non-blocking send）；`SearchWorker` 在獨立 tokio task 執行。
- [ ] `search.query` handler 只做 send，立即回傳 `{ "ok": true }`；結果透過 EventBus 推送。
- [ ] 驗收：`cargo test` 通過；搜尋速度/正確性不變；stale request_id 忽略不變。

### TD.4.C — TerminalService 獨立 Actor

- [ ] `TerminalActor` 以 `mpsc::Sender<TerminalCommand>` 接收 Spawn/Input/Resize/Close 指令。
- [ ] `TerminalHandler` 改為 send command 而非直接 lock manager；PTY output 仍透過 EventBus 推送。
- [ ] 驗收：`cargo test` 通過；terminal open/input/close 正常；LazyVim launch 不受影響。

---

## TD.5 — 安全性與品質（功能不變）

> 來源：技術債報告 §6 + §7。

### TD.5.A — 建立 npm run verify

- [ ] `package.json` 加入：`"check": "npm run lint && npm run build"`、`"rust:test"`、`"rust:clippy"`、`"verify": "npm run check && npm run test && npm run rust:test && npm run rust:clippy"`。
- [ ] 驗收：`npm run verify` 從頭到尾零錯誤。

### TD.5.B — CSP 與 Network Allowlist

- [ ] `NetworkPolicy` trait：`allow_url(purpose: NetworkPurpose, url: &Url) -> Result<(), PolicyError>`；purpose = AiProvider | Translation | WebSearch | ModelDownload。
- [ ] Agent web search / AI / translation / model download 呼叫前查詢 policy；`tauri.conf.json` CSP 與 Rust policy 同步。
- [ ] 驗收：`cargo test` 通過；未在 allowlist 的 URL 被拒絕並有明確 error。

### TD.5.C — 搜尋 keyboard navigation 狀態機測試

- [ ] `useLauncherKeyboard.test.ts`：Up/Down 邊界 wrap-around、Enter → onSelect、Escape → onClose、Tab 切換。
- [ ] `panelRouting.test.ts`：`CommandUiType::Panel(name)` → 正確 React component。
- [ ] 驗收：`npm run test` 通過。

### TD.5.D — 補充前端 chunk merge 測試

- [ ] `mergeSearchResults` replace:true 覆蓋（不追加）；replace:false 合併去重（path 為 key）。
- [ ] Stale request_id 的 chunk 到達時忽略，不更新 UI。
- [ ] 驗收：`npm run test` 通過。

---

## Blocked

- **Phase 5.5.D**（platform sandbox）— Research branch only。ADR-027 批准前不解封 generic shell tool。
- **TD.4.B** SearchService actor：需等 TD.2.A（AppContainer）完成後才能乾淨抽出 SearchWorker。
- **TD.4.C** TerminalService actor：需等 TD.2.B（FeatureModule trait）完成後再拆。

---

## 建議執行順序

```
TD.5.A → TD.1.A → TD.1.B → TD.1.C → TD.1.D → TD.1.E（分批）
  → TD.2.A → TD.2.B → TD.2.C
  → TD.3.A → TD.3.B → TD.3.C
  → TD.4.A → TD.4.B → TD.4.C
  → TD.5.B → TD.5.C+D
```

TD.1 各項可平行推進；TD.2 需在 TD.1 完成後才合理；TD.4 在 TD.2 之後。
每個子項完成後立即 commit。