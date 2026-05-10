# Keynova Tasks

> 完成任何批次後須更新 `docs/memory.md` 與對應 ADR。

---

## DOCS — 文件架構重組（依 ai_agent_adr_development_rules_v2.docx）

> **目標**：將目前散落在根目錄的 .md 文件，重組為 docx 規範的 `docs/` 結構，並將 `decisions.md` 拆分為獨立 ADR 檔案。
> **原則**：純文件改動，不修改任何程式碼，不影響任何功能。
> **批次順序**：每批完成後 commit，再進行下一批。

### 目標目錄結構

```
docs/
├── adr/
│   ├── 0000-template.md              ← ADR 標準模板（9 節）
│   ├── 0001-frontend-framework.md    ← ADR-001
│   ├── 0002-state-management.md      ← ADR-002
│   ├── 0003-styling.md               ← ADR-003
│   ├── 0004-backend-framework.md     ← ADR-004
│   ├── 0005-terminal.md              ← ADR-005
│   ├── 0006-dual-search-backend.md   ← ADR-006
│   ├── 0007-development-ide.md       ← ADR-007
│   ├── 0008-terminal-prewarm.md      ← ADR-008
│   ├── 0009-builtin-command-registry.md ← ADR-009
│   ├── 0010-config-manager-toml.md   ← ADR-010
│   ├── 0011-ai-provider-abstraction.md ← ADR-011
│   ├── 0012-note-sync-strategy.md    ← ADR-012
│   ├── 0013-system-control-permissions.md ← ADR-013
│   ├── 0014-action-knowledge-plugin-boundary.md ← ADR-014
│   ├── 0015-search-backend-preference.md ← ADR-015
│   ├── 0016-agent-read-only-tools.md ← ADR-016
│   ├── 0017-automation-executor.md   ← ADR-017
│   ├── 0018-app-module-split.md      ← ADR-018
│   ├── 0019-progressive-search-runtime.md ← ADR-019
│   ├── 0020-terminal-launch-specs.md ← ADR-020
│   ├── 0021-persisted-search-index.md ← ADR-021
│   ├── 0022-agent-approval-boundary.md ← ADR-022
│   ├── 0023-react-tool-schema-foundation.md ← ADR-023
│   ├── 0024-agent-system-indexer.md  ← ADR-024
│   ├── 0025-web-search-providers.md  ← ADR-025
│   ├── 0026-provider-driven-react-loop.md ← ADR-026
│   └── 0027-generic-shell-sandbox.md ← ADR-027
├── architecture.md                   ← 系統架構（新建）
├── CLAUDE.md                         ← AI agent 守則（從 docx 完整移入）
├── memory.md                         ← 從根目錄 memory.md 遷移
├── security.md                       ← 安全邊界（新建）
├── tasks.md                          ← 從根目錄 tasks.md 遷移
└── testing.md                        ← 測試策略（新建）
根目錄保留：
├── CLAUDE.md                         ← 更新路徑引用至 docs/
├── decisions.md                      ← 保留（作為 ADR index，內容精簡為 index）
├── memory.md                         ← 保留捷徑（或更新 CLAUDE.md 路徑後刪除）
└── tasks.md                          ← 保留捷徑（同上）
```

---

### DOCS.1 — 建立 docs/ 骨架

- [ ] 建立 `docs/` 目錄（新建一個空的 `.gitkeep` 或直接建子目錄）
- [ ] 建立 `docs/adr/` 目錄
- [ ] 驗收：`docs/` 與 `docs/adr/` 目錄存在

### DOCS.2 — 新建 `docs/adr/0000-template.md`

內容：docx §12「ADR 標準模板」的完整 9 節格式，含所有說明文字與欄位定義。

- [ ] 建立 `docs/adr/0000-template.md`，包含：
  - 標題格式、狀態、日期、決策者、相關文件欄位
  - § 1 Context（技術背景）— 需回答現在問題、資料規模、不處理的風險
  - § 2 Constraints（系統限制與邊界）— 平台、runtime、記憶體、安全、網路限制
  - § 3 Alternatives Considered（替代方案）— 至少兩方案、優缺點、效能分析（時間/空間/I/O/啟動/維護成本）、風險
  - § 4 Decision（最終決策）— 選擇、原因、犧牲、feature flag、migration、rollback
  - § 5 Consequences（系統影響）— 正面影響、負面影響/技術債、使用者/開發者/測試/安全影響
  - § 6 Implementation Plan（實作計畫）— 步驟列表，ADR 接受前不得修改正式程式碼
  - § 7 Rollback Plan（回滾策略）— feature flag、資料格式、migration rollback、殘留檔案
  - § 8 Validation Plan（驗證方式）— unit/integration/performance/security test、量測指標
  - § 9 Open Questions（未解問題）— 需要開發者決策的項目

### DOCS.3 — 新建 `docs/claude.md`（AI agent 守則）

內容：`ai_agent_adr_development_rules_v2.docx` **全文**，轉為 Markdown 格式，保留所有 23 節。

- [ ] 建立 `docs/claude.md`，包含：
  - § 0 專案文檔結構（docs/ 目錄表）
  - § 1 核心原則（安全性 > 資料正確性 > ... > 程式碼簡潔）
  - § 2 正式程式碼的定義（表格：通常視為正式 vs 通常不視為正式）
  - § 3 ADR 狀態與權限（5 種狀態表 + AI agent 權限說明）
  - § 4 ADR 判斷邊界與例外（2 欄對照表：不需要 ADR vs 必須建立 ADR）
  - § 5 ADR 強制觸發條件（5.1 核心依賴 / 5.2 模組通訊 / 5.3 演算法 / 5.4 安全邊界 / 5.5 資料模型）
  - § 6 ADR 阻塞範圍與任務拆分
  - § 7「停止實作」的定義
  - § 8 使用者要求跳過 ADR 時的處理
  - § 9 文件衝突解決順序
  - § 10 AI Agent 標準工作流程（7 步驟）
  - § 11 ADR 檔案命名規則
  - § 12 ADR 標準模板（9 節完整格式）
  - § 13 程式碼修改規則
  - § 14 測試規則（含測試類型表）
  - § 15 安全規則（15.1 檔案路徑 / 15.2 敏感資料 / 15.3 網路存取）
  - § 16 效能規則
  - § 17 文件同步規則（表格）
  - § 18 docs/tasks.md 建議格式
  - § 19 docs/memory.md 記憶規則
  - § 20 完成修改後的回報格式
  - § 21 禁止行為（清單）
  - § 22 ADR 建立後的標準回覆
  - § 23 ADR 被拒絕或取代時的處理

### DOCS.4 — 新建 `docs/architecture.md`

內容：從現有文件萃取系統整體架構、模組邊界、資料流、IPC、I/O、儲存設計。

- [ ] 建立 `docs/architecture.md`，包含：
  - **系統概覽**：6 層架構（Presentation / IPC Bridge / Core Framework / Business Logic / Indexer / Platform）
  - **模組邊界**：前端 src/ 結構、後端 src-tauri/src/ 結構
  - **資料流**：User Input → CommandPalette → IPC（cmd_dispatch）→ CommandRouter → Handler → Manager → EventBus → Frontend
  - **IPC 契約**：`cmd_dispatch(route: string, payload: JSON)` 統一入口、EventBus 事件 topic 列表
  - **儲存設計**：config.toml（ConfigManager）、knowledge.db（SQLite WAL）、notes/（Markdown files）、search/tantivy/（Tantivy index）、nvim/（portable）
  - **EventBus topics**：`terminal.output`、`search-results-chunk`、`agent-step`、`system-monitoring-tick`、`nvim-download-progress`、`ai-response`、`translation-result`
  - **搜尋架構**：Everything IPC（Windows）→ Tantivy → app-cache fallback；SystemIndexer（mdfind/plocate/ignore）
  - **Agent 架構**：ReAct loop（ToolCallProvider → tool dispatch → observation redaction → EventBus）、approval gate、audit log
  - **擴展模式**：CommandRouter（trait-based）、EventBus（broadcast）、ConfigManager（layered TOML）、BuiltinCommandRegistry、PanelRegistry

### DOCS.5 — 新建 `docs/testing.md`

內容：測試策略、指令、覆蓋要求。

- [ ] 建立 `docs/testing.md`，包含：
  - **測試指令**：`cargo test`、`cargo clippy -- -D warnings`、`npm run lint`、`npm run build`、`npm run test`（Vitest，計畫中）、`npm run verify`（計畫中）
  - **現有測試概況**：157+ Rust unit/integration tests；0 前端 Vitest（TD.1.D 計畫補）
  - **測試類型與使用時機**（表格）：unit / integration / regression / security / performance / migration
  - **優先覆蓋範圍**：核心行為、regression case、錯誤處理、安全邊界、資料格式相容性
  - **低價值測試禁止清單**：只測函式存在、只測 mock 被呼叫、只測 happy path、複製實作邏輯到測試
  - **平台限制**：sandbox tests（Windows Job Object 需真實 OS）、system_monitoring snapshot（需真實 sysinfo）
  - **慢速測試**：Tantivy index build、PTY spawn、reqwest blocking download
  - **計畫補測試範圍**（TD.1.D / TD.5.C+D）：前端 Vitest（searchRanking、inputMode、keyboard navigation、chunk merge）

### DOCS.6 — 新建 `docs/security.md`

內容：安全邊界、權限模型、敏感資料處理原則。

- [ ] 建立 `docs/security.md`，包含：
  - **安全優先順序**：安全性 > 資料正確性 > 可回滾性 > 可測試性 > 效能
  - **能力邊界（Agent）**：讀取 OK（keynova.search、filesystem.read 在 workspace root 內）；寫入 / shell 需 approval；generic shell 需 sandbox（ADR-027 blocked）
  - **檔案路徑處理規則**：不信任使用者 path、必須處理 `..`/symlink/absolute path/Windows drive prefix/Unicode normalization、避免 path traversal
  - **敏感資料規則**：API key、token、password、OAuth token 不得寫入 log/cache/test fixture；顯示時必須遮蔽（`sk-****`）
  - **網路存取規則**：涉及網路必須先建立 ADR，分析連線目標、離線運作、TLS、proxy、telemetry
  - **Sandbox 狀態**（ADR-027）：Windows Job Object OK（AppContainer 待辦）；Linux bwrap OK（seccomp 待辦）；macOS sandbox-exec OK（deprecated，App Sandbox 評估中）
  - **sensitive path 清單**：`/.ssh`、`/.gnupg`、`.env`、`id_rsa`、`id_ed25519`、`credentials`
  - **config 安全**：API key 存 ConfigManager，不暴露前端 bundle；SettingPanel 敏感設定遮蔽顯示
  - **IPC 邊界**：`cmd_dispatch` 為唯一前後端邊界；不允許前端直接呼叫 Tauri 底層 API（TD.1.A ESLint 限制）

### DOCS.7 — 拆分 `decisions.md` → `docs/adr/NNNN-*.md`（27 個檔案）

每個 ADR 獨立檔案，完整 9 節格式。以下為每個 ADR 的檔案名稱與內容來源：

| 檔案 | 來源 | 內容重點 |
|------|------|----------|
| `0001-frontend-framework.md` | ADR-001 | React 18 + TypeScript vs Vue/Svelte |
| `0002-state-management.md` | ADR-002 | Zustand vs Redux/Jotai |
| `0003-styling.md` | ADR-003 | Tailwind CSS vs CSS Modules/styled-components |
| `0004-backend-framework.md` | ADR-004 | Rust + Tauri 2.x vs Electron |
| `0005-terminal.md` | ADR-005 | xterm.js + PTY vs 自行實作 |
| `0006-dual-search-backend.md` | ADR-006 | Tantivy + Everything IPC 雙後端 |
| `0007-development-ide.md` | ADR-007 | JetBrains RustRover |
| `0008-terminal-prewarm.md` | ADR-008 | Pre-warm shell vs 按需 spawn |
| `0009-builtin-command-registry.md` | ADR-009 | BuiltinCommand trait + Registry |
| `0010-config-manager-toml.md` | ADR-010 | flat HashMap TOML vs typed struct（TD.3.C 計畫）|
| `0011-ai-provider-abstraction.md` | ADR-011 | AiProvider trait + EventBus 推送 |
| `0012-note-sync-strategy.md` | ADR-012 | Markdown file I/O vs SQLite vs CRDT |
| `0013-system-control-permissions.md` | ADR-013 | COM API / WMI / netsh 分工 |
| `0014-action-knowledge-plugin-boundary.md` | ADR-014 | ActionArena + KnowledgeStore + Plugin deny-by-default |
| `0015-search-backend-preference.md` | ADR-015 | `[search].backend` auto 選擇邏輯 |
| `0016-agent-read-only-tools.md` | ADR-016 | Agent 第一階段僅 read-only tools |
| `0017-automation-executor.md` | ADR-017 | `automation.execute` 走 cmd_dispatch 邊界 |
| `0018-app-module-split.md` | ADR-018 | lib.rs thin + app/ 子模組（TD.2 進一步拆 AppContainer）|
| `0019-progressive-search-runtime.md` | ADR-019 | stream=true + EventBus chunk（TD.4.B 計畫 actor）|
| `0020-terminal-launch-specs.md` | ADR-020 | `CommandUiType::Terminal(TerminalLaunchSpec)` |
| `0021-persisted-search-index.md` | ADR-021 | Tantivy on-disk index + Knowledge Store schema v2 |
| `0022-agent-approval-boundary.md` | ADR-022 | Approval state machine + prompt audit（TD.4.A 計畫 async channel）|
| `0023-react-tool-schema-foundation.md` | ADR-023 | schemars + AgentToolSpec + observation redaction |
| `0024-agent-system-indexer.md` | ADR-024 | SystemIndexer 抽象層（Everything/mdfind/plocate/ignore）|
| `0025-web-search-providers.md` | ADR-025 | SearXNG/Tavily/DuckDuckGo 分級 |
| `0026-provider-driven-react-loop.md` | ADR-026 | Provider-driven ReAct vs local heuristics |
| `0027-generic-shell-sandbox.md` | ADR-027 | Job Object/bwrap/sandbox-exec + 解封條件 |

- [ ] 建立所有 27 個 ADR 檔案（可分批：0001-0010、0011-0020、0021-0027）
- [ ] 每個檔案使用 9 節完整格式，填入現有 `decisions.md` 已有的資訊
- [ ] 缺少的節（如 Implementation Plan、Rollback Plan、Validation Plan）對已實作的 ADR 標記「已實作，無需回滾」或填寫實際驗收紀錄

### DOCS.8 — 將 `decisions.md` 改為 ADR Index

`decisions.md` 保留，但內容改為指向 `docs/adr/` 的索引頁，包含：
- 流程規則摘要（5 種狀態、ADR 觸發條件、禁止行為）
- 各 ADR 的一行摘要與連結
- 新建 ADR 的命名規則與標準回覆格式

- [ ] 改寫 `decisions.md` 為精簡 index（< 100 行）

### DOCS.9 — 遷移 `tasks.md` → `docs/tasks.md`

- [ ] 將本檔（tasks.md）複製至 `docs/tasks.md`
- [ ] 更新 `CLAUDE.md` 中所有 `tasks.md` 路徑引用改為 `docs/tasks.md`
- [ ] 根目錄 `tasks.md` 改為 1 行引用：`# 請見 docs/tasks.md`（保留供工具相容）

### DOCS.10 — 遷移 `memory.md` → `docs/memory.md`

- [ ] 將 `memory.md` 複製至 `docs/memory.md`
- [ ] 更新 `CLAUDE.md` 中所有 `memory.md` 路徑引用改為 `docs/memory.md`
- [ ] 根目錄 `memory.md` 改為 1 行引用

### DOCS.11 — 更新 `CLAUDE.md` 引用路徑

更新 CLAUDE.md 中的 Session 開場 / 收尾協議路徑，以及所有文件描述段落，改為 `docs/` 子路徑。

- [ ] Session 開場：`memory.md` → `docs/memory.md`、`tasks.md` → `docs/tasks.md`、`skill.md` → `docs/claude.md`
- [ ] Session 收尾：更新 `tasks.md`、`memory.md`、`decisions.md` 路徑
- [ ] Documentation 段落：補充 `docs/` 子目錄結構說明
- [ ] 加入說明：新 ADR 應建立於 `docs/adr/NNNN-title.md`，`decisions.md` 為 index

### DOCS.12 — 驗收清單

- [ ] `docs/adr/` 目錄存在且含 0000-template + 0001~0027 共 28 個檔案
- [ ] `docs/architecture.md` 存在且含 6 層架構說明
- [ ] `docs/claude.md` 存在且含完整 23 節
- [ ] `docs/testing.md` 存在且含測試指令與覆蓋要求
- [ ] `docs/security.md` 存在且含安全邊界與敏感資料規則
- [ ] `docs/tasks.md` 存在且與根目錄 `tasks.md` 一致
- [ ] `docs/memory.md` 存在且與根目錄 `memory.md` 一致
- [ ] `decisions.md` 改為精簡 index（< 100 行）
- [ ] `CLAUDE.md` 路徑引用全部更新
- [ ] `cargo test` + `npm run lint` + `npm run build` 通過（純文件改動，應全部通過）

---

### 建議執行順序

```
DOCS.1（建 docs/ 骨架）
  → DOCS.2（0000-template.md）
  → DOCS.3（docs/claude.md）
  → DOCS.4（docs/architecture.md）
  → DOCS.5（docs/testing.md）
  → DOCS.6（docs/security.md）
  → DOCS.7（docs/adr/ 27 個 ADR，分三批）
    → 批 A：0001~0010
    → 批 B：0011~0020
    → 批 C：0021~0027
  → DOCS.8（decisions.md → index）
  → DOCS.9（docs/tasks.md 遷移）
  → DOCS.10（docs/memory.md 遷移）
  → DOCS.11（CLAUDE.md 路徑更新）
  → DOCS.12（驗收）
```

---

---

## 最高原則：功能不變（Feature Freeze During Refactor）

**所有 TD 階段執行期間，下列功能的行為必須完整保留，任何改動前須通過對應驗收清單：**

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

## 已完成基線（Completed Baseline）

- [x] **Phase 0–3.8**：Tauri/React/Rust scaffold、CommandRouter/EventBus、launcher UI、app/file 搜尋、terminal、settings、hotkeys、mouse/system control、AI/translation/workspace/note/calculator/history/model 功能。
- [x] **BUG-1 ~ BUG-15**：包含 first-open lazy panel sizing、scrollbar refresh 等使用者端修復。
- [x] **Phase 4**：ActionRef/ActionArena、UiSearchItem、schema-driven settings、Workspace Context v2、Knowledge Store DB worker、Plugin manifest/permissions、Agent runtime/approval/audit/memory、Tantivy persisted index、workspace session binding、search stream/chunked progressive results、automation executor、app struct split。
- [x] **Phase 4.5 Agent**：transparency UI、read-only tools、approval state machine、prompt audit/context budget、medium/high-risk action scaffolding、SearXNG/Tavily/DuckDuckGo web search、ReAct safety foundation（schemars、AgentToolSpec、observation redaction、SystemIndexer）。
- [x] **Search ranking**：SearchPlan per-provider quotas、sort_balanced_truncate、WSL home enumeration、OneDrive/Dropbox dynamic discovery、28 Rust tests。
- [x] **Phase 5.1.A**：Agent frontend run ordering bug（`upsertRunChronologically` prepend）。
- [x] **Phase 5.1.B**：AI Chat message retain bug（`rposition()` 只刪最後一筆）。
- [x] **Phase 5.8**：Note name robustness（`validate_note_name()`、reserved name 拒絕）。
- [x] **Phase 5.2.A**：Search stream fairness（`applySourceQuotas()`、`replace:true` final chunk）。
- [x] **Phase 5.2.B**：Search stream diagnostics（`SearchChunkDiagnostics` + footer 顯示）。
- [x] **Phase 5.3.A+B+C**：Cross-platform config paths（`platform_dirs.rs`）、legacy path migration。
- [x] **Phase 5.4**：Default AI provider Ollama + SetupCard。
- [x] **Phase 5.5.A1~A4+B1~B3+C+D+E+F+G**：ReAct loop（ToolCallProvider trait、FakeProvider、loop skeleton、filesystem tools、git.status approval gate、offline fallback、sandbox infra、audit persist、frontend timeline、regression tests）。157 tests。
- [x] **Phase 5.7.A+B**：Tantivy index 統一（Agent/Launcher 共用）、non-Windows launcher file search。
- [x] **Phase 5.11.A~D**：AgentHandler 重構（extract_quoted 修正、工具常數、safety path check、agent/ 子目錄拆分、AgentError enum、ToolPermission gate、WebSearchProvider trait）。
- [x] **Phase 6.1**：`/model_remove` + ModelRemovePanel。
- [x] **Phase 6.2**：`/system_monitoring` panel（SystemMonitoringHandler、stream/stop、SystemMonitoringPanel、CPU/RAM/Disk/Network/Process、排序切換）。
- [x] **Phase 6.3**：Feature toggles（features.ai/agent 預設 false、backend guards、SettingPanel toggle UI）。
- [x] **Phase 6.4**：`/tr` 全語言支援（108 語言、LangPicker、SUPPORTED_LANGS）。
- [x] **Phase 6.5**：搜尋結果編碼修復（非 UTF-8 路徑過濾、前端 `hasEncodingError()`）。
- [x] **Phase 6.6**：LazyVim portable Neovim（`portable_nvim_manager`、`handlers/nvim`、`NvimDownloadPanel.tsx`）。
- [x] **Phase 6.7.A+B**：記憶體優化（Tantivy writer buffer 15MB + explicit drop、AppManager 回傳 `&[AppInfo]`）。

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
- [ ] External agent integration contract：定義最小 provider 介面，允許 plan + tool requests + Keynova-safe action proposals，且不繞過 approval/audit。
- [ ] Manual validation script：local search、feature search、private architecture denial、note draft approval、terminal approval、rejected approval、disabled web search、action payload handoff。

### 6.6 — 後續補強

- [ ] 可攜式 nvim 路徑寫入 config（`notes.nvim_bin`），使用者可在 `/setting` 覆蓋。

### 6.7.C+D — 記憶體優化（後半）

- [ ] **C — Handler 懶初始化**：`PortableNvimManager`、`SystemMonitoringHandler` 的 sysinfo `System` 物件改為 `OnceLock` 或 on-demand init，不在 `AppState::new()` 啟動時預建。`ModelManager` Ollama catalog 改為 lazy fetch。
- [ ] **D — sysinfo 按需建立**（已有 6.2 設計遵守，驗收待量測）：冷啟動 WorkingSet 降至 120MB 以下（使用 Task Manager 量測）。

### 5.9 — Plugin & WASM Runtime

- [ ] WASM runtime proof-of-concept。
- [ ] Host Functions API：`log`、`get_setting`、`search_workspace`、`read_clipboard`、`http_fetch`、`run_action`。
- [ ] Plugin action/search provider 註冊。
- [ ] Plugin settings schema。
- [ ] Plugin audit log 寫入 Knowledge Store。
- [ ] Hot reload。

### 5.10 — Future Features

- [ ] Smart Clipboard：classify code/url/path/email/json/error log/command/markdown/image，提供 actions。
- [ ] Dev Workflow Pack：git status/branch、cargo test、npm build、open project、search symbol、explain compiler error、create commit message。
- [ ] Command chaining：`/search error | ai explain | note append`。
- [ ] Universal Command Bar：跨 app/file/note/history/command/action，AI fallback。
- [ ] 維護面板 first-open / ESC / resize regression checklist。
- [ ] 手動驗收 /note LazyVim：`/note`、`/note lazyvim`、`/note lazyvim foo`、Vim 內 Esc、terminal exit shortcut。

---

## TD.1 — 前端邊界重構（功能不變，只切邊界）

> **來源**：技術債報告 §8 第 1 階段 + §4 P0 + §7 品質防線。
> **原則**：不改任何行為。每個子項可獨立 merge。

### TD.1.A — 統一前端 IPC Client

**問題**：`useIPC.ts` 有錯誤 normalization，`useCommands.ts` 直接呼叫 `invoke`，兩處錯誤格式不一致。

**做法**：
- [ ] 新建 `src/ipc/client.ts`：
  ```ts
  export async function dispatch<T>(route: string, payload?: unknown): Promise<T>
  export async function dispatchVoid(route: string, payload?: unknown): Promise<void>
  ```
  內部呼叫 `invoke("cmd_dispatch", { route, payload: payload ?? null })`，統一錯誤格式（`{ code, message }`）。
- [ ] 將 `useIPC.ts` 與 `useCommands.ts` 改為 re-export `dispatch`，不直接 import `invoke`。
- [ ] 將所有現有元件中直接用 `invoke` 的地方改為 import `dispatch`（`CommandPalette.tsx`、`AiPanel.tsx`、`SystemMonitoringPanel.tsx`、`NvimDownloadPanel.tsx` 等）。
- [ ] 驗收：`grep -r "from \"@tauri-apps/api/core\"" src/` 只剩 `src/ipc/client.ts`。
- [ ] `npm run lint && npm run build` 通過。

### TD.1.B — ESLint 限制直接 invoke

**問題**：沒有靜態防線，未來開發者會再次直接用 invoke。

**做法**：
- [ ] 在 `.eslintrc.cjs`（或 `eslint.config.js`）加入：
  ```js
  "no-restricted-imports": ["error", {
    paths: [{
      name: "@tauri-apps/api/core",
      importNames: ["invoke"],
      message: "Use src/ipc/client.ts instead of calling invoke directly."
    }]
  }]
  ```
- [ ] 確認 `npm run lint` 通過（TD.1.A 完成後才能通過）。

### TD.1.C — CommandPalette 搜尋邏輯提取為純函式

**問題**：`CommandPalette.tsx` 同時處理 UI、IPC、搜尋串流、keyboard state、terminal、panel、metadata、icon 與 window resize，難以測試與維護。

**目標**：把純運算邏輯搬出，不改 UI 行為。

**做法**：
- [ ] 建立 `src/features/launcher/domain/searchRanking.ts`：
  - `sortSearchResults(items: SearchResult[]): SearchResult[]`（現有排序邏輯）
  - `applySourceQuotas(items: SearchResult[], limit: number): SearchResult[]`（現有 quota 邏輯）
  - `mergeSearchResults(existing: SearchResult[], incoming: SearchResult[], replace: boolean): SearchResult[]`（現有 merge 邏輯）
- [ ] 建立 `src/features/launcher/domain/inputMode.ts`：
  - `parseInputMode(query: string): InputMode`（`command | search | empty`）
  - `InputMode` type / union
- [ ] `CommandPalette.tsx` 改為 import 並呼叫上述純函式，不重複邏輯。
- [ ] 驗收：純函式所在檔案無任何 Tauri API import；`npm run build` 通過。

### TD.1.D — 補 Vitest 單元測試

**問題**：前端純邏輯無測試覆蓋，重構時無安全網。

**做法**：
- [ ] 安裝 Vitest（若未安裝）：`npm i -D vitest`，在 `package.json` 加 `"test": "vitest run"`。
- [ ] 新建 `src/features/launcher/domain/__tests__/searchRanking.test.ts`，覆蓋：
  - [ ] `sortSearchResults`：app > command > note > file 排序正確。
  - [ ] `applySourceQuotas`：各來源上限（app/command/note:8, history:12, model:6, file:unbounded）。
  - [ ] `mergeSearchResults`：`replace:true` 直接替換、`replace:false` 合併去重。
- [ ] 新建 `src/features/launcher/domain/__tests__/inputMode.test.ts`，覆蓋：
  - [ ] `/` 開頭 → command mode。
  - [ ] 空字串 → empty mode。
  - [ ] 一般文字 → search mode。
- [ ] `npm run test` 通過（新加的測試）。

### TD.1.E — 拆分 CommandPalette hooks

**問題**：IPC、搜尋串流、window size、metadata、icon 全混在 CommandPalette component 內。

**做法**（每個 hook 獨立 PR）：
- [ ] `src/features/launcher/hooks/useLauncherSearch.ts`：封裝 `search.query` 呼叫、request-id 生成、chunk listener、stale-request 忽略、loading state、cancel on unmount。`CommandPalette` 改為呼叫此 hook。
- [ ] `src/features/launcher/hooks/useLauncherWindowSize.ts`：封裝 `Tauri.window.getCurrent().innerSize()` 與 resize listener，隔離 Tauri window 依賴。
- [ ] `src/features/launcher/hooks/useSearchMetadata.ts`：封裝 `search.metadata` 惰性載入邏輯（hover/select 觸發，有 cache）。
- [ ] `src/features/launcher/hooks/useSearchIcons.ts`：封裝 `search.icon` 惰性載入，icon_key → data URI cache。
- [ ] `src/features/launcher/hooks/useLauncherKeyboard.ts`：封裝鍵盤導航狀態機（Up/Down/Enter/Escape/Tab），只依賴 props/callbacks，無 IPC。
- [ ] 每個 hook 完成後：`CommandPalette.tsx` import 替換、`npm run build` 通過、搜尋手動回歸。

---

## TD.2 — 後端 Composition Root 重構（功能不變）

> **來源**：技術債報告 §4 P0（AppState::new() 過度集中）+ §5.1 + §8 第 2 階段。
> **原則**：不改任何 handler 行為，只改組裝方式。

### TD.2.A — 建立 app/container.rs

**問題**：`AppState::new()` 同時建立 16+ 個 manager 與 handler，新增功能一定修改同一檔案。

**做法**：
- [ ] 新建 `src-tauri/src/app/container.rs`：
  ```rust
  pub struct AppContainer {
      pub event_bus: EventBus,
      pub action_arena: Arc<ActionArena>,
      pub knowledge_store: KnowledgeStoreHandle,
      // shared services:
      pub config: Arc<Mutex<ConfigManager>>,
      pub note_manager: Arc<Mutex<NoteManager>>,
      pub history_manager: Arc<Mutex<HistoryManager>>,
      pub search_manager: Arc<Mutex<SearchManager>>,
      pub workspace_manager: Arc<Mutex<WorkspaceManager>>,
      pub model_manager: Arc<ModelManager>,
      pub ai_manager: Arc<AiManager>,
      pub agent_runtime: Arc<AgentRuntime>,
      pub translation_manager: Arc<TranslationManager>,
      pub terminal_manager: Arc<Mutex<TerminalManager>>,
  }
  impl AppContainer {
      pub fn build() -> Self { /* 現有 AppState::new() manager 建立邏輯移至此 */ }
  }
  ```
- [ ] `AppState::new()` 改為：
  ```rust
  pub fn new() -> Self {
      let container = AppContainer::build();
      let mut router = CommandRouter::new();
      register_all_handlers(&mut router, &container);
      Self { command_router: router, ... }
  }
  ```
- [ ] 驗收：`AppState::new()` 主體行數 < 30 行；`cargo test` 通過。

### TD.2.B — 建立 FeatureModule trait

**問題**：新增 handler 必須修改 `AppState::new()` 主體，沒有統一的註冊介面。

**做法**：
- [ ] 新建 `src-tauri/src/app/modules.rs`：
  ```rust
  pub trait FeatureModule: Send + Sync {
      fn register_handlers(&self, router: &mut CommandRouter, container: &AppContainer);
  }
  ```
- [ ] 將現有 handler 逐一包裝成 `FeatureModule` 實作（可從 `SearchModule`、`AgentModule`、`TerminalModule` 三個最複雜的先做）。
- [ ] `register_all_handlers()` 接受 `&[Box<dyn FeatureModule>]` 並依序呼叫。
- [ ] 驗收：新增 feature 只需新建 module struct + 在 module list 加一行；`cargo test` 通過。

### TD.2.C — Builtin Command Registry 整理

**問題**：`BuiltinCommandRegistry` 在 `AppState::new()` 中以 `lock().expect("registry init")` 手動逐一 `register`，與 `FeatureModule` 模式不一致。

**做法**：
- [ ] 讓每個 BuiltinCommand 透過 `FeatureModule::register_handlers` 自行註冊，不在 `AppState::new()` 中手動列舉。
- [ ] `BuiltinCmdHandler` 依賴的 `BuiltinCommandRegistry` 保留，但 registry 填充改在各 module 的 `register_handlers` 內完成。
- [ ] 驗收：`AppState::new()` 中移除所有 `reg.register(...)` 呼叫；`cargo test` 通過。

---

## TD.3 — Typed IPC & DTO（功能不變）

> **來源**：技術債報告 §4 P0（IPC 型別遺失）+ §5.2 + §8 第 3 階段。
> **原則**：保留相容層（`cmd_dispatch` 仍接受舊格式），新增 typed DTO 作為並行路徑，不破壞現有呼叫。

### TD.3.A — 後端建立 ipc/routes.rs 與 DTO

**做法**：
- [ ] 新建 `src-tauri/src/ipc/mod.rs` 與 `src-tauri/src/ipc/dto/`:
  ```rust
  // ipc/dto/search.rs
  #[derive(Deserialize)]
  pub struct SearchQueryRequest {
      pub query: String,
      pub limit: usize,
      pub stream: bool,
      pub request_id: Option<String>,
      pub first_batch_limit: Option<usize>,
  }
  // ipc/dto/agent.rs
  #[derive(Deserialize)]
  pub struct AgentStartRequest {
      pub prompt: String,
      pub workspace_id: Option<String>,
  }
  // ipc/dto/setting.rs
  #[derive(Deserialize)]
  pub struct SettingSetRequest {
      pub key: String,
      pub value: String,
  }
  ```
- [ ] 在對應 handler 中新增 typed parse helper，解析失敗回傳標準化 `{ "error": "invalid_payload", "field": "..." }`。
- [ ] 現有 `Value` 解析路徑保留（相容舊前端）；DTO 為可選升級路徑。
- [ ] 先轉換：`search.query`、`agent.start`、`setting.set`。
- [ ] 驗收：`cargo test` 通過；payload parse error 有一致 error code。

### TD.3.B — 前端建立 ipc/routes.ts 與 DTO 型別

**做法**：
- [ ] 新建 `src/ipc/routes.ts`：
  ```ts
  export const ROUTES = {
    SEARCH_QUERY: "search.query",
    AGENT_START: "agent.start",
    SETTING_SET: "setting.set",
    // ... 所有 route 常數
  } as const;
  ```
- [ ] 新建 `src/ipc/dto/search.ts`、`src/ipc/dto/agent.ts`、`src/ipc/dto/setting.ts`，與後端 DTO 對齊。
- [ ] `dispatch<T>(ROUTES.SEARCH_QUERY, { query, limit, stream, request_id })` 取代 `dispatch("search.query", {...})`。
- [ ] 驗收：route 字串不再在元件層出現；`npm run build` 通過。

### TD.3.C — ConfigManager Typed Config Snapshot

**問題**：TOML 被 flatten 成 `HashMap<String, String>`，bool/int/float 靠字串猜測，unknown key 無法驗證。

**做法**：
- [ ] 新建 `AppConfig` typed struct（`src-tauri/src/core/typed_config.rs`）：
  ```rust
  #[derive(Deserialize, Default)]
  pub struct AppConfig {
      pub features: FeaturesConfig,
      pub search: SearchConfig,
      pub notes: NotesConfig,
      pub ai: AiConfig,
      // ...
  }
  ```
- [ ] `ConfigManager` 新增 `typed_snapshot() -> Arc<AppConfig>`（read-only，從現有 HashMap parse，不移除舊 HashMap）。
- [ ] 各 handler 改用 `typed_snapshot()` 讀取 features.* / search.* 等 bool/int 設定，不再用 `.get("...").parse::<bool>()`。
- [ ] 驗收：`cargo test` 通過；bool/int 設定不再靠字串猜測。

---

## TD.4 — Actor-Like Services（功能不變）

> **來源**：技術債報告 §5.1（feature module）+ §6.2（Agent async channel）+ §6.3（Mutex 慢操作）+ §8 第 4 階段。
> **原則**：Service 對外 API 不變，只換內部實作。

### TD.4.A — Agent Approval 改成 Async Channel

**問題**：目前 Agent ReAct loop 使用 polling approval，cancellation 與 timeout 難以測試。

**做法**：
- [ ] 建立 `AgentRunHandle`：
  ```rust
  pub struct AgentRunHandle {
      pub cancel_tx: watch::Sender<bool>,
      pub approval_tx: mpsc::Sender<ApprovalDecision>,
  }
  ```
- [ ] `spawn_react_loop` 改為 tokio async task，使用 `cancel_tx` cancel token 與 `approval_tx` approval channel。
- [ ] `approve_run` / `reject_run` 對應 handler 改為傳送至 channel。
- [ ] 新增單元測試：approval timeout（channel dropped）→ run 取消；cancel signal → loop 停止。
- [ ] 驗收：`cargo test` 通過；手動：Agent run 仍可 approve/reject/cancel；audit log 仍寫入。

### TD.4.B — SearchService 獨立 Worker

**問題**：`SearchManager` 在 handler thread 內直接做 IO（Tantivy、Everything IPC、file cache），遇到慢操作會佔用 lock。

**做法**：
- [ ] 建立 `SearchService`（actor-like）：
  ```
  UI / IPC
    ↓ SearchRequest { query, limit, stream, request_id }
  CommandRouter
    ↓
  SearchService.send(req)   ← non-blocking，立即回傳 Ok
    ↓ (background worker)
  SearchWorker              ← 真正執行搜尋
    ↓
  EventBus emits search.results.chunk
  ```
- [ ] `SearchService` 持有 `mpsc::Sender<SearchRequest>`，不再持有 `Mutex<SearchManager>`。
- [ ] `SearchWorker` 在獨立 tokio task 執行，持有 `SearchManager` + cancel token per request_id。
- [ ] `search.query` handler 只做 send，立即回傳 `{ "ok": true }`；結果透過 EventBus 推送。
- [ ] 驗收：`cargo test` 通過；手動：搜尋速度/正確性不變；stale request_id 忽略行為不變。

### TD.4.C — TerminalService 獨立 Actor

**問題**：`TerminalManager` 以 `Arc<Mutex<TerminalManager>>` 持有所有 PTY session，spawn/input/resize 等操作在 lock 內執行。

**做法**：
- [ ] 建立 `TerminalActor`，以 `mpsc::Sender<TerminalCommand>` 接收指令：
  ```rust
  enum TerminalCommand {
      Spawn(TerminalLaunchSpec, oneshot::Sender<Result<String>>),
      Input(String, Vec<u8>),
      Resize(String, u16, u16),
      Close(String),
  }
  ```
- [ ] `TerminalHandler` 改為 send command 而非直接 lock manager。
- [ ] PTY output callback 仍透過 EventBus 推送（行為不變）。
- [ ] 驗收：`cargo test` 通過；手動：terminal open/input/close 正常；LazyVim launch 不受影響。

---

## TD.5 — 安全性與品質（功能不變）

> **來源**：技術債報告 §6（P2 安全性與穩定性）+ §7（品質防線）。

### TD.5.A — 建立 npm run verify

**做法**：
- [ ] 在 `package.json` 加入：
  ```json
  "scripts": {
    "check": "npm run lint && npm run build",
    "test": "vitest run",
    "rust:test": "cargo test --manifest-path src-tauri/Cargo.toml",
    "rust:clippy": "cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings",
    "verify": "npm run check && npm run test && npm run rust:test && npm run rust:clippy"
  }
  ```
- [ ] 驗收：`npm run verify` 從頭到尾零錯誤。

### TD.5.B — CSP 與 Network Allowlist

**問題**：Tauri CSP 允許 localhost 與多個外部 API，network allowlist 沒有集中管理。

**做法**：
- [ ] 建立 `NetworkPolicy` trait：
  ```rust
  pub trait NetworkPolicy {
      fn allow_url(&self, purpose: NetworkPurpose, url: &Url) -> Result<(), PolicyError>;
  }
  enum NetworkPurpose { AiProvider, Translation, WebSearch, ModelDownload }
  ```
- [ ] Agent web search / AI provider / translation / model download 呼叫前查詢 policy。
- [ ] `tauri.conf.json` CSP 與 Rust policy 保持同步。
- [ ] 驗收：`cargo test` 通過；未在 allowlist 的 URL 被拒絕並有明確 error。

### TD.5.C — 搜尋 keyboard navigation 狀態機測試

**問題**：技術債報告 §7.3 指出 keyboard navigation 狀態轉移是優先需補測試的邏輯。

**做法**：
- [ ] 在 `src/features/launcher/hooks/__tests__/useLauncherKeyboard.test.ts` 補：
  - [ ] Up/Down 正確移動 selected index，邊界 wrap-around。
  - [ ] Enter 觸發 onSelect callback。
  - [ ] Escape 觸發 onClose callback。
  - [ ] Tab 在 command suggestions 與 search results 間切換。
- [ ] Panel routing 測試：`src/features/launcher/__tests__/panelRouting.test.ts`（`CommandUiType::Panel(name)` → 正確 React component）。
- [ ] 驗收：`npm run test` 通過。

### TD.5.D — 補充前端 chunk merge 測試

**做法**：
- [ ] `mergeSearchResults` replace:true 覆蓋現有結果（不追加）。
- [ ] `mergeSearchResults` replace:false 合併去重（path 為 key）。
- [ ] Stale request_id：舊 request_id 的 chunk 到達時忽略，不更新 UI 狀態。
- [ ] 驗收：`npm run test` 通過。

---

## Blocked

- **Phase 5.5.D**（platform sandbox）— Research branch only。ADR-027 批准前不解封 generic shell tool。
- **TD.4.B SearchService actor**：需等 TD.2.A（AppContainer）完成後才能乾淨地抽出 SearchWorker。
- **TD.4.C TerminalService actor**：需等 TD.2.B（FeatureModule trait）完成後再拆。

---

## 建議執行順序

```
TD.5.A (verify script)
  → TD.1.A (ipc/client.ts)
  → TD.1.B (eslint no-restricted-imports)
  → TD.1.C (domain pure functions)
  → TD.1.D (Vitest)
  → TD.1.E (hook split, 分批)
  → TD.2.A (AppContainer)
  → TD.2.B (FeatureModule trait)
  → TD.2.C (builtin registry)
  → TD.3.A (backend DTO)
  → TD.3.B (frontend routes.ts)
  → TD.3.C (typed config snapshot)
  → TD.4.A (agent async channel)
  → TD.4.B (search worker)
  → TD.4.C (terminal actor)
  → TD.5.B (CSP/network policy)
  → TD.5.C+D (keyboard/chunk tests)
```

TD.1 各項可平行推進；TD.2 需在 TD.1 完成後才合理；TD.4 在 TD.2 之後。
每個子項完成後立即 commit，不等整個 phase 完成再一次提交。