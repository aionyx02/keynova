# decisions.md — 架構決策紀錄（ADR）

> 格式依 `ai_agent_adr_development_rules_v2.docx` v2 規範。

---

## ADR 流程規則（摘要）

### 核心原則
- 安全性 > 資料正確性 > 可回滾性 > 可測試性 > 效能 > 開發速度 > 程式碼簡潔。
- ADR 管理會影響長期架構、效能、安全、資料格式、核心依賴或 public contract 的決策，不阻塞小型局部可回滾修改。
- **AI agent 不得自行將 ADR 狀態改為「接受」**。只有開發者可以接受 ADR。
- 若不確定是否需要 ADR，優先建立草案，不直接實作。

### ADR 狀態定義

| 狀態 | 意義 | AI agent 權限 |
|------|------|--------------|
| 提議 | 尚未允許實作 | 可以建立 |
| 接受 | 開發者已審閱，可開始實作 | 不得自行設定 |
| 拒絕 | 此方案被拒絕 | 不得實作 |
| 廢棄 | 曾接受但已不再適用 | 不得依此實作 |
| 取代 | 被新 ADR 取代 | 必須遵守新的已接受 ADR |

### ADR 強制觸發條件（摘要）

必須建立 ADR 的情況：
- 引入或替換核心依賴（資料庫、搜尋引擎、序列化格式、native binary）。
- 改變模組間通訊或 async/sync 模型（IPC、queue、event bus、actor、deadlock 風險）。
- 新增或改寫核心搜尋 / 索引 / 同步 / cache / 排程演算法（O(n log n) 以上且在熱路徑）。
- 改變安全邊界或檔案 I/O 權限（使用者可控路徑讀寫、sandbox、allowlist）。
- 改變資料模型、持久化格式或 public contract（schema、migration、CLI flag）。

通常不需要 ADR：
- 修正 typo、補測試、補文件。
- 單一模組內部 private helper 小型重構。
- 不改 public API 的小型 bug fix。
- 僅用於測試 / lint / format 的非 runtime 依賴。

### ADR 命名規則
```
docs/adr/NNNN-short-kebab-case-title.md
```
編號遞增四位數，標題英文 kebab-case，內文可用中文。

### ADR 標準模板（九節）
> 本節為新 ADR 的撰寫參考，既有 ADR 依可用資料填寫。

```
## ADR-NNN — [標題]

**狀態：** 提議 | 接受 | 拒絕 | 廢棄 | 取代
**日期：** YYYY-MM-DD
**決策者：** 開發者 | AI agent | 團隊
**相關文件：**
- docs/architecture.md

### Context（技術背景）
現在的問題、為何現有設計不足、資料規模、風險。

### Constraints（系統限制）
平台、runtime、記憶體、啟動時間、安全、網路等限制。

### Alternatives Considered（替代方案）
至少兩個方案，含優缺點與效能分析。

### Decision（最終決策）
選擇哪個方案、原因、犧牲、migration / rollback 需求。

### Consequences（影響與副作用）
正面影響、負面影響 / 技術債、使用者 / 開發者 / 測試 / 安全影響。

### Implementation Plan（實作計畫）
列出步驟，ADR 接受前不得修改正式程式碼。

### Rollback Plan（回滾策略）
如何退回原狀、feature flag、資料格式、殘留檔案。

### Validation Plan（驗證方式）
unit / integration / performance / security test，量測指標。

### Open Questions（未解問題）
需要開發者決策的項目。
```

---

## 禁止行為

- ADR 未接受前實作受限正式功能。
- 自行把 ADR 狀態改成接受，或用「Accepted」、「approved」等同義詞繞過審閱。
- 引入依賴但不說明理由。
- 改變安全邊界但不更新 security.md。
- 改變資料格式但不提供 migration 或 rollback。
- 大規模重構但沒有任務或 ADR。
- 靜默忽略 failing tests 或編造測試結果。

---

## 文件衝突解決順序

1. 開發者在當前對話中的明確指示
2. 已接受的 ADR
3. docs/security.md
4. docs/architecture.md
5. docs/claude.md（CLAUDE.md）
6. docs/testing.md
7. docs/tasks.md（tasks.md）
8. docs/memory.md（memory.md）

---

## 既有 ADR 決策紀錄

> 以下 ADR 均為 **Status: Accepted**，已於對應 Phase 實作完成。
> 日期欄位為實作完成估計日；歷史 ADR 依可用資料填寫各節，不補推測內容。

---

## ADR-001 — 前端框架

**狀態：** 接受  
**日期：** 2026-05-01  
**決策者：** 開發者

### Context
需為 Tauri 桌面 App 選擇前端框架，確保生態成熟且開發效率高。

### Alternatives Considered
- **React 18 + TypeScript**：生態最成熟，Tauri 官方範本支援完善，TypeScript 保證型別安全。
- **Vue 3**：可行，但團隊熟悉度較低。
- **Svelte**：包體積小，但生態與第三方 UI library 較少。

### Decision
採用 React 18 + TypeScript。

### Consequences
- Tauri 官方範本直接支援，零額外設定。
- 大量 React ecosystem（hooks、zustand、xterm.js）可直接使用。
- 需維護 TypeScript 型別定義與 tsconfig。

---

## ADR-002 — 狀態管理

**狀態：** 接受  
**日期：** 2026-05-01  
**決策者：** 開發者

### Context
需為前端選擇全域狀態管理，確保輕量且適合小中型桌面 App 快速迭代。

### Alternatives Considered
- **Zustand**：輕量，無 boilerplate，心智負擔低。
- **Redux Toolkit**：生態強但較重，適合大型多人協作專案。
- **Jotai**：原子模型，可行但較少見。

### Decision
採用 Zustand。

### Consequences
- Store 定義極簡，無 action / reducer 分離開銷。
- 需手動管理 slice 邊界，大型專案可能需拆多個 store。

---

## ADR-003 — 樣式方案

**狀態：** 接受  
**日期：** 2026-05-01  
**決策者：** 開發者

### Context
需要能快速建置一致 UI 的樣式方案，減少命名與設計 token 管理成本。

### Alternatives Considered
- **Tailwind CSS**：utility-first，快速迭代，設計 token 統一。
- **CSS Modules**：隔離性好但命名成本高。
- **styled-components**：JS-in-CSS，運行時成本高。

### Decision
採用 Tailwind CSS。

### Consequences
- className 字串較長，需靠 IDE 自動補全。
- 主題色 / spacing token 集中於 tailwind.config，改動方便。

---

## ADR-004 — 後端與桌面框架

**狀態：** 接受  
**日期：** 2026-05-01  
**決策者：** 開發者

### Context
需選擇桌面 App 框架，目標包體積小、記憶體佔用低、安全性高。

### Alternatives Considered
- **Rust + Tauri 2.x**：WebView2 / WKWebView，打包體積 ~50MB，記憶體低，安全 IPC model。
- **Electron**：生態強，但 Chromium 打包體積 ~200MB，記憶體佔用高。

### Decision
採用 Rust + Tauri 2.x。

### Consequences
- 後端邏輯以 Rust 撰寫，型別安全且效能高。
- Tauri IPC 層（`cmd_dispatch`）為前後端唯一通訊邊界。
- 跨平台需各自處理 Windows / macOS / Linux 差異。

---

## ADR-005 — 終端方案

**狀態：** 接受  
**日期：** 2026-05-01  
**決策者：** 開發者

### Context
需在前端嵌入全功能終端，支援 ANSI、PTY、跨平台。

### Alternatives Considered
- **xterm.js + PTY（portable-pty）**：跨平台成熟方案，社群維護活躍，支援 ConPTY（Windows）。
- **自行實作終端**：成本極高且風險大。

### Decision
採用 xterm.js + portable-pty，Windows 啟用 `windowsPty: { backend: "conpty" }`。

### Consequences
- 前端 xterm.js 處理 rendering，後端 PTY 處理 I/O。
- PTY 讀取在獨立 thread，output 透過 EventBus 推送前端。

---

## ADR-006 — 文件搜尋雙後端

**狀態：** 接受  
**日期：** 2026-05-02  
**決策者：** 開發者

### Context
需提供跨平台檔案搜尋，同時允許 Windows 使用者享受 Everything 的即時搜尋體驗。

### Constraints
- 跨平台：Windows / macOS / Linux 均需可用。
- Windows 使用者可能已安裝 Everything；非 Windows 沒有 Everything。
- 搜尋後端切換不能影響 IPC contract。

### Alternatives Considered
- **Tantivy only**：跨平台，但 Windows 不能享用 Everything 速度優勢。
- **Everything only**：Windows 專屬，非 Windows 無法使用。
- **雙後端（Tantivy + Everything IPC）**：最大相容性，Everything 缺失時自動 fallback。

### Decision
預設 Tantivy；Windows 若偵測到 Everything 服務可用則優先使用 Everything IPC，失敗時自動 fallback Tantivy。

### Consequences
- `SearchResult` 包含 `backend` 欄位。
- `SearchManager` 實作 backend 偵測與切換。
- Windows 專屬 `everything_ipc.rs` 提供可用性偵測與查詢 API。

### Open Questions
- Everything IPC 完整實作需補官方 SDK 細節與錯誤處理策略（目前為佔位符）。

---

## ADR-007 — 開發 IDE

**狀態：** 接受  
**日期：** 2026-05-01  
**決策者：** 開發者

### Decision
使用 JetBrains RustRover 作為主要 IDE。

### Consequences
- `.idea/`、`*.iml`、`*.iws`、`*.ipr` 全列入 `.gitignore`，不提交任何 IDE 設定。

---

## ADR-008 — Terminal PTY Pre-warm 策略

**狀態：** 接受  
**日期：** 2026-05-03  
**決策者：** 開發者

### Context
Windows shell 同步 spawn 需 200–800ms，導致終端初始延遲 > 1 秒，影響使用者體驗。

### Constraints
- Windows 預設 shell：PowerShell / pwsh（可用 `KEYNOVA_TERMINAL_SHELL` 覆寫）。
- PTY spawn 為 blocking 操作，不可在 Mutex 鎖內執行。
- WSL 需繼承 `TERM=xterm-256color` / `COLORTERM=truecolor` 等 capability hints。

### Alternatives Considered
- **每次按需 spawn**：簡單但延遲高（>200ms）。
- **Pre-warm 一個 shell**：啟動時背景 spawn，claim 後補充下一個，第一次開啟幾乎零延遲（<50ms）。

### Decision
應用啟動時在背景線程預先 spawn 一個 shell（pre-warm），等待使用者輸入 `>` 時直接取用。取用後立即補充下一個 warm session。

### Consequences
- `TerminalManager` 新增 `warm: Option<WarmEntry>` 欄位。
- `terminal.open` 回傳型別從 `String` 改為 `{ id, initial_output }`。
- `terminal.open` 接收前端 xterm rows/cols，claim warm session 後立即 resize。
- `AppState` 新增短時間 focus guard，避免 ESC 從 terminal 回 search 時被 WebView2 blur-to-hide 誤關閉。
- blocking 的 `openpty + spawn_command` 必須在 `Mutex<TerminalManager>` 鎖外執行。

---

## ADR-009 — BuiltinCommand 可擴展指令 Registry

**狀態：** 接受  
**日期：** 2026-05-02  
**決策者：** 開發者

### Context
需在不修改 core 的情況下動態新增內建指令（`/help`、`/setting`、`/note` 等），並供前端動態渲染指令清單。

### Decision
在 `core/builtin_command_registry.rs` 定義 `BuiltinCommand` trait + `BuiltinCommandRegistry`；具體指令實作於 `handlers/builtin_cmd.rs`。

### Consequences
- 新增指令 = 實作 trait + 注冊，不修改 core。
- `cmd.list` IPC 供前端動態渲染。
- 新增 `models/builtin_command.rs`（`CommandUiType::Inline|Panel`、`BuiltinCommandResult`）。
- 前端 `PanelRegistry` 以 `Record<string, ComponentType>` 映射 panel name → React 元件，新增 panel 零前端 core 改動。

---

## ADR-010 — ConfigManager TOML I/O

**狀態：** 接受  
**日期：** 2026-05-02  
**決策者：** 開發者

### Context
需要一個可讀寫、型別安全的 config 管理，支援分 section 的 TOML 格式，且允許 runtime reload。

### Constraints
- Config 存放於 OS 標準目錄（`keynova_config_dir()`），不硬編碼 APPDATA。
- 使用者設定優先，缺少時 fallback 至 `default_config.toml`（bundle 內）。

### Decision
以 `toml` crate 讀寫 `config.toml`；內部以 flat dot-key `HashMap<String, String>` 表示，讀取時遞迴攤平 TOML Table，寫回時重建 `[section]` 結構並以 `toml_value_str()` 偵測型別。

### Consequences
- Flat HashMap 使 `setting.get/set` IPC 介面簡單，鍵名可直接作為 UI label。
- Bool/int/float 靠字串猜測型別（TD.3.C 計畫改善為 typed AppConfig struct）。
- 前端 `SettingPanel` 四 tab 呈現，onBlur 才呼叫 `setting.set`（非 onChange）。

### Open Questions
- TD.3.C：`AppConfig` typed struct 計畫，移除 bool/int 靠字串猜測的問題。

---

## ADR-011 — AI Provider 抽象化

**狀態：** 接受  
**日期：** 2026-05-03  
**決策者：** 開發者

### Context
需支援多個 AI provider（Ollama、OpenAI-compatible、未來 Claude API），並允許 local-first 操作。

### Constraints
- API key 不得暴露於前端 bundle。
- reqwest blocking 不可阻塞 tokio executor。
- 多輪對話歷史保留最近 20 訊息，避免 token overflow。

### Alternatives Considered
- **Trait + provider adapters**：`AiProvider` trait，各 provider 實作，EventBus 非同步推送。允許未來切換 Gemini / GPT / 本地模型。
- **硬編碼 Claude API**：最快，但無法切換 provider。

### Decision
以 `AiProvider` / `TranslationProvider` trait 定義介面；結果透過 `EventBus → Tauri event → 前端 listen` 非同步推送。local Ollama 為預設 provider（ADR-026 決策）。

### Consequences
- API key 存於 ConfigManager（`ai.api_key`），不硬編碼。
- 請求 ID 由前端產生，後端 event payload 帶回，前端對應 pending 狀態。
- timeout 預設 30 秒，可透過 `ai.timeout_secs` 設定。

---

## ADR-012 — 筆記同步策略

**狀態：** 接受  
**日期：** 2026-05-03  
**決策者：** 開發者

### Context
需選擇筆記儲存策略，確保簡單可靠，且不引入同步衝突問題。

### Alternatives Considered
- **Markdown 檔案儲存（純 file I/O）**：簡單可靠，可用任何編輯器開啟，無衝突。
- **SQLite 儲存**：搜尋效率高但增加複雜度。
- **CRDT / OT 即時同步**：複雜度遠超 MVP 需求。

### Decision
筆記以 Markdown 檔案儲存於 `keynova_data_dir()/notes/`。前端每次開啟時讀取，儲存時立即寫回。外部編輯器修改由使用者手動重新整理（MVP 不做即時監看）。

### Consequences
- `managers/note_manager.rs`：純 file I/O，filename = `sanitize(name) + ".md"`。
- 前端 `NoteEditor.tsx`：auto-save（800ms debounce）+ Ctrl+S 立即儲存。
- 進階需求（OneDrive/Obsidian 整合）留在未來 Phase。

---

## ADR-013 — System Control 權限範圍

**狀態：** 接受  
**日期：** 2026-05-03  
**決策者：** 開發者

### Constraints
- 亮度 API（IOCTL）需要管理員權限，不適合標準使用者環境。
- WiFi 切換需要 UAC；MVP 僅顯示狀態。
- Tauri 沙盒不需特別開放（音量/亮度由 Rust backend 處理，非 WebView API）。

### Decision
- 音量：Win32 `IAudioEndpointVolume` COM API。
- 亮度：PowerShell WMI 呼叫（標準使用者可執行）。
- WiFi：`netsh wlan show interfaces`（僅顯示狀態）。
- 所有平台特定實作包在 `#[cfg(target_os)]`，非 Windows 回傳 `NotImplemented`。

### Consequences
- `managers/system_manager.rs`：Windows 實作 + stub。
- `handlers/system_control.rs`：namespace `"system"`，含 `volume.get/set/mute`、`brightness.get/set`、`wifi.info`。

---

## ADR-014 — Phase 4 Action / Knowledge / Plugin 邊界

**狀態：** 接受  
**日期：** 2026-05-05  
**決策者：** 開發者

### Context
隨著 Agent 與 Plugin 加入，需明確定義能力邊界，防止高風險能力擴散。

### Constraints
- 搜尋熱路徑不應跨 IPC 傳送肥大的 JSON action payload。
- SQLite 不可進入 Tauri command/search hot path。
- Agent 與 Plugin 若直接取得 shell/filesystem/network/AI key 會形成高風險能力擴散。

### Decision
- 前端只接收 `UiSearchItem` + `ActionRef`，完整 action payload 留在 Rust `ActionArena`。
- SQLite 透過 `KnowledgeStoreHandle` dedicated worker thread 存取。
- Agent 與 Plugin Runtime 預設 deny 高風險能力，所有本機動作必須走 Action System 或 Host Functions。

### Consequences
- 新增 `core/action_registry.rs`（ActionRef + ActionArena）。
- 新增 `core/knowledge_store.rs`（rusqlite + bounded channel + WAL）。
- Knowledge Store write path 支援 action/clipboard batch insert；shutdown 前先 flush DB worker。
- `SettingPanel` 改由 schema 驅動，敏感設定遮蔽顯示。

---

## ADR-015 — Search Backend Preference 與重建入口

**狀態：** 接受  
**日期：** 2026-05-05  
**決策者：** 開發者

### Decision
搜尋後端以 `[search].backend` 設定表達偏好（`auto` / `everything` / `app_cache` / `tantivy`），由 `SearchManager::select_backend` 依可用性決定 active backend；`search.backend` 回傳 structured debug info，`/rebuild_search_index` 先重建目前 Windows file cache。

### Consequences
- `auto` 讓 Windows 使用者可優先走 Everything，缺少 DLL/服務時仍有 fallback。
- backend selection 純函式化後可用測試鎖住回退規則。
- `default_config.toml` 與 settings schema 新增 `[search]` keys。
- `CommandPalette` 顯示 search backend debug badge。

---

## ADR-016 — Agent Read-only Tool Runtime

**狀態：** 接受  
**日期：** 2026-05-05  
**決策者：** 開發者

### Context
`/ai` Agent 第一階段需明確限制 tool 範圍，避免在 approval/audit 機制尚未完善前開放寫入或 shell 能力。

### Decision
`agent.tool` 支援 `keynova.search`（本機搜尋 grounding）與 `web.search`（SearXNG JSON，預設 disabled）；不執行本機寫入或 shell。

### Consequences
- `keynova.search` 安全讀取 workspace summary、command metadata、非敏感 settings schema、model catalog、notes/history 摘要。
- `web.search` 只回 title/url/snippet，送出前拒絕 `private_architecture` / `secret` query term。
- `handlers/agent.rs` 注入 config/note/history/workspace/command/model context。

---

## ADR-017 — Automation Action Chain Executor

**狀態：** 接受  
**日期：** 2026-05-06  
**決策者：** 開發者

### Context
需提供 workflow 執行能力，但不能讓 AutomationHandler 持有或繞過各 handler 的權限邊界。

### Decision
`automation.execute` 走既有 `cmd_dispatch` 邊界逐步執行 workflow actions，明確拒絕 recursive `automation.execute`。

### Consequences
- `WorkflowExecutionReport` 回傳整體 log 與逐步 action execution。
- `AutomationEngine::execute` 為純 Rust executor，可用 closure 測試。

---

## ADR-018 — App Module Split

**狀態：** 接受  
**日期：** 2026-05-06  
**決策者：** 開發者

### Context
`lib.rs` 同時承載 state、IPC dispatch、watchers、tray、shortcut、window、control plane，後續 Phase 會讓入口檔難以維護。

### Decision
`src-tauri/src/lib.rs` 保持 thin entrypoint，將 app runtime 分成 `app/state.rs`、`app/bootstrap.rs`、`app/dispatch.rs`、`app/control_server.rs`、`app/watchers.rs`、`app/shortcuts.rs`、`app/tray.rs`、`app/window.rs`。

### Consequences
- `AppState` 改為 `pub(crate)` app state，跨 app modules 共享。
- Tauri command macro wrapper 保留在 `bootstrap.rs`，實際 command logic 委派 `dispatch.rs`。
- **TD.2**：下一步計畫進一步提取 `AppContainer` 與 `FeatureModule` trait，讓 `AppState::new()` 不直接 new 所有 manager。

---

## ADR-019 — Progressive Search Runtime

**狀態：** 接受  
**日期：** 2026-05-06  
**決策者：** 開發者

### Context
第一批 UI 不應被 Everything/file backend 阻塞；需要前端可取消 stale search。

### Decision
`search.query` 保留同步相容路徑，新增 `stream=true` 模式；stream 模式先回傳 app/command/note/history/model 的 quick batch，再由背景 worker 透過 `search.results.chunk` EventBus topic 發送後續結果。

### Consequences
- file provider 透過 timeout boundary 包起來；timeout 時回報 `timed_out_providers`。
- `search.metadata` 成為 lazy metadata/preview 入口。
- request id + backend generation 可讓前後端同時丟棄 stale search。
- **TD.4.B**：計畫進一步改成 SearchService actor（非同步 worker + cancel token）。

---

## ADR-020 — Terminal Launch Specs For Builtin Commands

**狀態：** 接受  
**日期：** 2026-05-06  
**決策者：** 開發者

### Context
`/note lazyvim` 需要直接啟動 nvim 透過 PTY，而非包在 default shell 中。

### Decision
`BuiltinCommandResult` 支援 `CommandUiType::Terminal(TerminalLaunchSpec)`。第一個消費者是 `/note lazyvim`，直接透過 PTY launch `nvim`。

### Consequences
- Terminal-backed command results 攜帶 `launch_id`、program、args、cwd、title、editor-session metadata。
- `terminal.open` 接受可選 `launch_spec`；plain terminal 仍走 pre-warmed shell path。
- Editor sessions 保留 Escape 給 vim，使用 `Ctrl+Shift+Q` 或 `Ctrl+Alt+Esc` 退出 launcher panel。
- LazyVim terminal env 涵蓋 `NVIM_APPNAME`、`XDG_CONFIG_HOME`、`XDG_DATA_HOME`、`XDG_STATE_HOME`、`XDG_CACHE_HOME`（project-local `.keynova/lazyvim/`）。

---

## ADR-021 — Persisted Search Index And Runtime Ranking

**狀態：** 接受  
**日期：** 2026-05-07  
**決策者：** 開發者

### Context
Tantivy 先前只作為 backend 偏好標籤，需建立真實 on-disk persisted index 以提高搜尋速度。

### Decision
Keynova 以 Tantivy 為 persisted on-disk search provider。Windows rebuild path 保留 file-cache scan 為 source of truth，snapshot 後寫入 Tantivy index（`name`、`path`、`kind`、folder fields）。

### Consequences
- `[search].index_dir` 可覆寫預設 index 位置。
- `auto` backend 選擇：Tantivy 有 documents 時優先，否則 fallback app-cache。
- Result rows 只回 `icon_key`；selected row lazy-load icon assets 透過 `search.icon`。
- Knowledge Store schema versioning（SQLite `user_version = 2`）在升級前建立 sibling `backups/` 複本。

---

## ADR-022 — Agent Approval Boundary And Prompt Audit

**狀態：** 接受  
**日期：** 2026-05-07  
**決策者：** 開發者

### Context
Agent execution 需要明確的 approval state machine，且每次 run 需記錄 prompt audit 以確保透明度。

### Decision
Batch 4 agent execution 使用 explicit approval state machine 和 agent-owned planned actions。每個 run 記錄 prompt audit（context budget、allowlisted sources、filtered private/secret sources）。

### Consequences
- `agent.start` 可回傳 `waiting_approval` runs，帶 typed planned actions（medium/high-risk）。
- Approved actions 回傳 UI-safe `BuiltinCommandResult`，AI panel 可開啟 note/setting/system/model panel 或 terminal。
- High-risk file writes 保持 scaffolded 而非直接執行。
- Knowledge Store schema version 3 新增 `agent_audit_logs` 和 `agent_memories`；長期記憶為 opt-in。
- **TD.4.A**：計畫將 approval polling 改為 async channel（`AgentRunHandle` + cancel token）。

---

## ADR-023 — ReAct Tool Schema And Observation Safety Foundation

**狀態：** 接受  
**日期：** 2026-05-08  
**決策者：** 開發者

### Context
在接線 provider-driven ReAct loop 之前，需先確保 Agent tools 是 typed、schema-generated 且 observation-bounded，防止 unbounded stdout/search/file observations 進入 LLM context。

### Constraints
- Tool parameter schemas 需從 Rust structs 自動生成（不手寫 JSON schema）。
- Observation 必須通過 redaction/truncation pipeline，不能 bypass visibility policy。
- OpenAI-compatible 與 local Ollama 為第一批 ReAct adapter 目標。

### Decision
Batch 4.6 先建立 tool typed / schema-generated / observation-bounded foundation，再接 provider-driven ReAct loop。`AgentToolSpec` 攜帶 LLM-safe tool name、description、generated schemas、approval policy、visibility policy、timeout、result limit、observation redaction policy。

### Alternatives Considered
- **直接接 ReAct loop（無 typed foundation）**：開發快但 observation unbounded，LLM 可能收到敏感內容。
- **Typed foundation 優先**：多一個 sprint，但安全基礎明確，後續工具可快速加入。

### Consequences
- Generic shell execution 不在第一批 ReAct tool set（等 ADR-027 sandbox）。
- `git.status` 等 typed tools 必須 approval-gated，不可透過 legacy `agent.tool` path 呼叫。
- Tool observations 先通過 redaction（likely secret lines 遮蔽、hard line/character cap、head+tail 保留）。

---

## ADR-024 — Agent SystemIndexer Search Path

**狀態：** 接受  
**日期：** 2026-05-08  
**決策者：** 開發者

### Context
Agent filesystem search 需要跨平台統一介面，並在各平台使用最佳索引器（Windows Everything / macOS mdfind / Linux plocate）。

### Decision
Agent filesystem search 透過 `SystemIndexer` 抽象層，而非直接遞迴掃描。Windows：Everything IPC → Tantivy；macOS/Linux：固定 CLI indexer（`mdfind`、`plocate`、`locate`）透過 bounded stdout parsing；fallback：bounded parallel traversal（`ignore` crate）。

### Consequences
- Agent filesystem results 包含 diagnostics（provider、fallback reason、visited count、permission-denied count、timeout、index age）。
- Empty 或 missing CLI indexer results 不作為 final truth（fallback roots 可用時繼續嘗試）。
- Fallback traversal 遵守 ignore filters，bounded by visit count + timeout。

---

## ADR-025 — Structured Agent Web Search Providers

**狀態：** 接受  
**日期：** 2026-05-08  
**決策者：** 開發者

### Context
Agent web search 需要結構化 provider 以避免抓取完整 HTML 頁面；DuckDuckGo 不提供結構化 API，應限於 best-effort fallback。

### Decision
Agent web search 預設 disabled，需配置結構化 provider 才啟用。SearXNG JSON 與 Tavily API 為結構化 adapter；DuckDuckGo HTML 僅作為 explicit best-effort fallback。

### Consequences
- `agent.web_search_provider`：`disabled` | `searxng` | `tavily` | `duckduckgo`。
- 所有 web results normalize 為 grounded title/url/snippet，送出前通過 query redaction。
- Tavily 使用 `agent.web_search_api_key`；SearXNG 使用 `agent.searxng_url`。

---

## ADR-026 — Provider-Driven ReAct Agent Loop

**狀態：** 接受  
**日期：** 2026-05-08  
**決策者：** 開發者

### Context
Agent mode 需從 local prompt heuristics 升級為 provider-driven ReAct loop，讓 AI provider 決定是否回答或請求 tool calls。

### Constraints
- ReAct loop 必須使用 user 選定的 AI provider/model，不強制走 API-only path。
- local Ollama 必須是 first-class Agent backend。
- Tool observations 只在通過 redaction/truncation 後才進入 LLM context。

### Alternatives Considered
- **Local heuristics only**：無 provider 依賴，但 Agent 能力受限，無法真正 ReAct。
- **Provider-driven ReAct**：LLM 決定 tool calls，heuristics 降為 offline fallback。犧牲：provider 不支援 tool calls 時 fallback 到 heuristics。

### Decision
移至 provider-driven ReAct loop。Rust 持有 context filtering、tool schema generation、policy、approval、execution、observation redaction、cancellation、timeout、max-step limits 與 audit logging。

### Consequences
- Local prompt heuristics 成為 offline/fallback 行為，非主要 Agent planner。
- Provider adapters normalize model responses 為 final text 或 typed tool calls。
- Unsupported provider/tool-call 組合必須明確 fail 或 fallback 到 safe chat/heuristics。
- **TD.4.A**：計畫將 approval polling 改為 async channel 以提升可測試性。

---

## ADR-027 — Generic Shell Tool Sandbox Requirement

**狀態：** 接受（research phase 完成 — product 尚未解封）  
**日期：** 2026-05-09  
**決策者：** 開發者

### Context
approval + regex/string deny list 不足以安全執行 generic shell。需要真實 platform sandbox 才能解封。

### Constraints
- Windows：Job Object 可滿足，但網路隔離需 AppContainer（尚未實作）。
- Linux：`bwrap` 可滿足，但 seccomp filter 尚未編譯；`bwrap` 缺失時必須明確拒絕（不 fallback 到 unsandboxed）。
- macOS：`sandbox-exec` deny-by-default profile 可滿足，但已被 Apple 標記 deprecated；長期需評估 App Sandbox entitlement。

### Alternatives Considered
- **Approval + deny list only**：快速，但正規表達式過濾無法覆蓋所有 shell bypass 技巧。
- **Platform sandbox（Job Object / bwrap / sandbox-exec）**：更強的隔離保證，但各平台實作不同且需要額外測試。

### Decision
Keynova 不暴露 generic `execute_shell_command` / `execute_bash_command` Agent tool，直到各平台真實 sandbox 存在。第一批 ReAct tools 為 narrow typed tools（固定 binary + structured args）。

### Research Findings（2026-05-09，`managers/sandbox_manager.rs`）

| 平台 | 機制 | 狀態 | 剩餘工作 |
|------|------|------|----------|
| Windows | Job Object（`CreateJobObjectW` + `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`）+ `WaitForSingleObject` timeout | 5/5 tests pass | AppContainer 網路隔離 |
| Linux | `bwrap --unshare-net --unshare-pid --unshare-ipc`；缺 bwrap 時明確拒絕 | 可行 | seccomp filter compilation；bwrap CI check |
| macOS | `sandbox-exec` deny-by-default `.sb` temp profile（per invocation，執行後刪除）| 可行（deprecated API）| App Sandbox entitlement 評估 |
| All | `output_cap_bytes` 截斷保護；caller 須接 `core::agent_observation` redaction | 實作完成 | 全平台 ReAct loop 整合測試 |

### Consequences
- 所有平台禁止 `sh -c`、`cmd /c`、shell-string 執行作為 generic Agent tool。
- Denied tool attempts 作為 observation 回傳 LLM，讓模型 self-correct 到 typed read-only tools。
- `sandbox_available()` 回傳 `Available | BinaryMissing | Unsupported` 三態。

### Open Questions
- Windows AppContainer 或 network job restriction 何時實作？
- Linux seccomp filter 是否需要 CI pre-compilation？
- macOS App Sandbox entitlement path 是否可替代 `sandbox-exec`？
- 全平台 integration test 在真實 ReAct loop 下的計畫？

---

## 待補強事項

- Everything IPC 完整實作尚需參考官方 SDK 細節與錯誤處理策略（`everything_ipc.rs` 目前為佔位符）。
- ADR-027 product 解封前需完成各平台剩餘工作（見 Open Questions）。
- TD.2（AppContainer / FeatureModule）、TD.3（Typed IPC/DTO）、TD.4（Actor-Like Services）為下一批架構演進，屆時需新增對應 ADR。
- 後續新增 ADR 時，請依 `ADR-0NNN` 連號並遵守第 12 節標準模板（9 節格式）。