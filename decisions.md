# decisions.md — 架構決策紀錄（ADR）

## ADR-017 Automation Action Chain Executor

- 狀態：Accepted
- 決策：`automation.execute` 走既有 `cmd_dispatch` 邊界逐步執行 workflow actions，而不是讓 `AutomationHandler` 自己持有或繞過各 handler。
- 理由：
  - Automation 應重用 Action/Command/Setting/Search 等既有權限與錯誤邊界。
  - 每一步需要可觀測的 output/error，失敗時停止，方便未來接 audit log 與 GUI workflow builder。
  - 明確拒絕 recursive `automation.execute`，避免 workflow 自我呼叫造成無界遞迴。
- 影響：
  - `WorkflowExecutionReport` 回傳整體 log 與逐步 action execution。
  - `AutomationEngine::execute` 是純 Rust executor，可用 closure 測試，也可由 Tauri dispatch 注入真實 action runner。

## ADR-018 App Module Split

- 狀態：Accepted
- 決策：`src-tauri/src/lib.rs` 保持 thin entrypoint，將 app runtime 分成 `app/state.rs`、`app/bootstrap.rs`、`app/dispatch.rs`、`app/control_server.rs`、`app/watchers.rs`、`app/shortcuts.rs`、`app/tray.rs`、`app/window.rs`。
- 理由：
  - `lib.rs` 已同時承載 state、IPC dispatch、watchers、tray、shortcut、window、control plane，後續 Phase 4 搜尋/Agent/Plugin 會讓入口檔難以維護。
  - Tauri command macro wrapper 保留在 `bootstrap.rs`，實際 command logic 委派到 `dispatch.rs`，避免 macro scope 問題，也讓 dispatch 可被 automation/action 重用。
- 影響：
  - `AppState` 改為 `pub(crate)` app state，跨 app modules 共享。
  - `lib.rs` 只保留 module 宣告與 root `run()` delegate。

## ADR-019 Progressive Search Runtime

- 狀態：Accepted
- 決策：`search.query` 保留同步相容路徑，新增 `stream=true` 模式；stream 模式先回傳 app/command/note/history/model 的 quick batch，再由背景 worker 透過 `search.results.chunk` EventBus topic 發送後續結果。
- 理由：
  - 第一批 UI 不應被 Everything/file backend 或未來 plugin provider 阻塞。
  - EventBus 已橋接成 Tauri frontend events，可重用 `search-results-chunk` legacy topic。
  - request id + backend generation 可以讓前端與後端同時丟棄 stale search。
- 影響：
  - file provider 透過 timeout boundary 包起來；timeout 時回報 `timed_out_providers` 而不是卡住 UI。
  - `search.metadata` 成為 lazy metadata/preview 入口，避免搜尋結果 payload 變肥。
  - `tantivy` backend preference 暫時走 rebuilt file cache 的 offline provider path；真 Tantivy persisted index 另列待辦。

> 本檔用於記錄關鍵技術決策、原因與替代方案。
> 2026-05-01：此檔於 Tauri 初始化過程中被覆蓋後重建，請在後續審查時確認內容是否與原版一致。

## ADR-001 前端框架

- 狀態：Accepted
- 決策：採用 React 18 + TypeScript
- 原因：生態成熟、Tauri 官方範本支援完善、團隊熟悉度高
- 替代方案：Vue / Svelte（可行但非目前優先）

## ADR-002 狀態管理

- 狀態：Accepted
- 決策：採用 Zustand
- 原因：輕量、心智負擔低、對小中型桌面 App 迭代快
- 替代方案：Redux Toolkit（較重）、Jotai（可行）

## ADR-003 樣式方案

- 狀態：Accepted
- 決策：採用 Tailwind CSS
- 原因：快速建置 UI、減少命名成本、便於一致化設計 token
- 替代方案：CSS Modules / styled-components

## ADR-004 後端與桌面框架

- 狀態：Accepted
- 決策：Rust + Tauri 2.x
- 原因：效能與記憶體佔用佳、打包體積小、安全性高
- 替代方案：Electron（生態強但包體積較大）

## ADR-005 終端方案

- 狀態：Accepted
- 決策：xterm.js + PTY
- 原因：跨平台成熟方案、社群維護活躍
- 替代方案：自行實作終端（成本高且風險大）

## ADR-006 文件搜尋雙後端

- 狀態：Accepted
- 決策：預設使用 tantivy；Windows 若偵測到 Everything 服務可用則優先使用 Everything IPC，失敗時自動回退 tantivy
- 原因：
  - 跨平台需要穩定可用的內建索引能力（tantivy）
  - Windows 使用者可在安裝 Everything 後取得更快搜尋體驗
  - 雙後端設計可降低單一依賴風險
- 影響：
  - `SearchResult` 需包含 `backend` 欄位
  - `SearchManager` 需實作 backend 偵測與切換
  - Windows 專屬 `everything_ipc.rs` 需提供可用性偵測與查詢 API

## ADR-007 開發 IDE

- 狀態：Accepted
- 決策：使用 JetBrains RustRover 作為主要 IDE
- 原因：對 Rust 有原生語義理解（非依賴 rust-analyzer 插件）、內建 Cargo task、Clippy 即時警告、Tauri 相關 JSON schema 提示；Windows 11 上開發體驗最佳
- 影響：
  - `.idea/` 整個目錄列入 `.gitignore`，不提交任何 IDE 設定
  - `*.iml` / `*.iws` / `*.ipr` 一併忽略

## ADR-008 Terminal PTY Pre-warm 策略

- 狀態：Accepted
- 決策：應用啟動時在背景線程中預先 spawn 一個 shell，Windows 預設優先使用 PowerShell/pwsh（可用 `KEYNOVA_TERMINAL_SHELL` 覆寫），等待使用者輸入 `>` 時直接取用
- 原因：
  - Windows shell 同步 spawn 需 200-800ms，導致終端初始延遲 >1 秒
  - Windows Terminal 使用者預期 `clear`、ANSI、PowerShell 命令可直接使用；cmd.exe 不符合此預期
  - WSL 需繼承 `TERM=xterm-256color` / `COLORTERM=truecolor` 等 terminal capability hints，避免 prompt / cursor 格式錯位
  - Pre-warm 使第一次開啟終端幾乎零延遲（<50ms）
  - 取用 warm session 後立即補充下一個，確保重複開啟也快速
- 關鍵設計約束：
  - blocking 的 `openpty + spawn_command` 必須在 `Mutex<TerminalManager>` 鎖外執行，避免阻塞其他 IPC 呼叫
  - Warm session reader thread 先 buffer 輸出（shell prompt），claim 後切換為 live EventBus 模式
  - `terminal.open` 回傳型別從 `String` 改為 `{ id, initial_output }`，前端可立即寫入初始 prompt
  - `terminal.open` 接收前端 xterm rows/cols；claim warm session 後立即 resize backend，避免 WSL/ConPTY 使用錯誤欄寬
  - xterm.js 啟用 `windowsPty: { backend: "conpty" }`，與 Windows ConPTY 行為對齊
- 影響：
  - `TerminalManager` 新增 `warm: Option<WarmEntry>` 欄位
  - `AppState` 新增 `terminal_manager` 欄位以供 setup 觸發 pre-warm
  - `terminal.open` IPC 回傳型別改變（前後端需同步）
  - `AppState` 新增短時間 focus guard，避免 ESC 從 terminal 回 search 時被 WebView2 blur-to-hide 誤關閉

## ADR-009 BuiltinCommand 可擴展指令 Registry

- 狀態：Accepted
- 決策：在 `core/builtin_command_registry.rs` 定義 `BuiltinCommand` trait + `BuiltinCommandRegistry`；具體指令（HelpCommand、SettingCommand）實作於 `handlers/builtin_cmd.rs`
- 原因：
  - 遵循專案既有擴展模式：新功能 = 實作 trait + 注冊，不修改 core
  - 指令清單（`cmd.list`）可供前端動態渲染，無需前後端硬編碼
  - `/help` 的執行在 handler 內特殊處理，避免 `Mutex<BuiltinCommandRegistry>` 鎖重入
- 影響：
  - 新增 `models/builtin_command.rs`（CommandUiType::Inline|Panel，BuiltinCommandResult）
  - `lib.rs` AppState 持有 `Arc<Mutex<BuiltinCommandRegistry>>`
  - 前端 `PanelRegistry` 以 Record 形式映射 panel name → React 元件，擴展零前端改動

## ADR-010 ConfigManager TOML I/O

- 狀態：Accepted
- 決策：以 `toml` crate 讀寫 `%APPDATA%\Keynova\config.toml`；內部以 flat dot-key HashMap 表示（如 `hotkeys.app_launcher`），讀取時遞迴攤平 TOML Table，寫回時重建 [section] 結構並以 `toml_value_str()` 偵測型別（數字/bool 不加引號）
- 原因：
  - 避免引入 JSON 之外的額外序列化格式；TOML 與既有 `default_config.toml` 格式一致
  - Flat HashMap 使 `setting.get/set` IPC 介面簡單，鍵名可直接作為 UI label
  - 使用者設定優先；缺少時 fallback 至 `default_config.toml`（套件內）
- 影響：
  - `core/config_manager.rs` 完全取代先前的 stub
  - `handlers/setting.rs` 新增，提供 `setting.get/set/list_all` IPC
  - 前端 `SettingPanel` 分四 tab 呈現，onBlur 才呼叫 `setting.set` IPC（非 onChange）

## ADR-011 AI Provider 抽象化

- 狀態：Accepted
- 決策：以 `AiProvider` / `TranslationProvider` trait 定義介面；MVP 實作為 `ClaudeProvider`（reqwest blocking + std::thread）；結果透過 `EventBus → Tauri event → 前端 listen` 非同步推送
- 原因：
  - Claude 是目前最佳選擇，但日後可能切換至 Gemini / GPT / 本地模型
  - reqwest blocking 運行於 std::thread，不阻塞 tokio executor（CommandHandler::execute 同步呼叫即可）
  - EventBus 推送模式已在 terminal.output 驗證，複用現有基礎設施
- 關鍵設計約束：
  - API key 存於 ConfigManager（`ai.api_key`），不硬編碼，不暴露於前端 bundle
  - 請求 ID（`request_id`）由前端產生，後端 event payload 帶回，前端用來對應 pending 狀態
  - 多輪對話歷史保留最近 20 訊息，避免 token overflow
  - timeout 預設 30 秒，可透過 `ai.timeout_secs` 設定
- 影響：
  - `managers/ai_manager.rs`、`managers/translation_manager.rs`：各持 publish_event callback
  - `handlers/ai.rs`、`handlers/translation.rs`：注入 ConfigManager 讀取 API key
  - 前端 `useAi.ts`、`useHistory.ts` listen `ai-response` / `translation-result` 事件

## ADR-012 筆記同步策略

- 狀態：Accepted
- 決策：筆記以 Markdown 檔案儲存於 `%APPDATA%\Keynova\notes\`；前端 NoteEditor 每次開啟時從 IPC 讀取，儲存時立即寫回；外部編輯器修改由使用者手動重新整理（MVP 不做即時監看）
- 原因：
  - 簡單可靠：無資料庫、無同步衝突、可直接用任何編輯器開啟
  - CRDT / OT 等即時同步方案複雜度遠超 MVP 需求
  - 進階需求（OneDrive/Obsidian 整合）留在 Phase 4
- 影響：
  - `managers/note_manager.rs`：純 file I/O，filename = sanitize(name) + ".md"
  - `handlers/note.rs`：list / get / save / create / delete / rename IPC
  - 前端 `NoteEditor.tsx` 含 auto-save（800ms debounce）+ Ctrl+S 立即儲存

## ADR-013 System Control 權限範圍

- 狀態：Accepted
- 決策：音量控制使用 Win32 `IAudioEndpointVolume` COM API；亮度透過 PowerShell WMI 呼叫（非 Win32 直接調用）；WiFi 狀態透過 `netsh wlan show interfaces`；所有平台特定實作包在 `#[cfg(target_os)]` 後，非 Windows 回傳 `NotImplemented` 錯誤
- 原因：
  - COM API 音量控制是 Windows 官方推薦方案，無需額外驅動
  - 亮度 API（IOCTL_VIDEO_QUERY_SUPPORTED_BRIGHTNESS 等）需要管理員權限；PowerShell WMI 方案在標準使用者下可運行
  - WiFi 連線管理（切換網路）需要 UAC；MVP 僅顯示狀態，不執行連線
  - Tauri 沙盒（capabilities）不需特別開放，因為音量/亮度均由 Rust backend 處理，非 WebView API
- 影響：
  - `managers/system_manager.rs`：Windows 實作 + stub
- `handlers/system_control.rs`：namespace "system"，含 volume.get/set/mute、brightness.get/set、wifi.info
- lib.rs 中原有的 inline `SystemHandler`（只有 ping）已由 `SystemControlHandler` 完全取代

## ADR-014 Phase 4 Action / Knowledge / Plugin 邊界

- 狀態：Accepted
- 決策：Phase 4 採漸進式收斂：前端只接收 `UiSearchItem` + `ActionRef`，完整 action payload 留在 Rust `ActionArena`；SQLite 透過 `KnowledgeStoreHandle` dedicated worker thread 存取；Agent 與 Plugin Runtime 預設 deny 高風險能力，所有本機動作必須走 Action System 或 Host Functions。
- 原因：
  - 搜尋熱路徑不應跨 IPC 傳送肥大的 JSON action payload
  - SQLite 不可進入 Tauri command/search hot path，避免 UI 卡頓
  - `/ai` Agent 與 Plugin 若直接取得 shell/filesystem/network/AI key 會形成高風險能力擴散
  - ActionRef + audit log 能讓 Automation、Agent、Plugin 共用同一套執行與審計模型
- 影響：
  - 新增 `models/action.rs`、`core/action_registry.rs`、`action.run` / `action.list_secondary`
  - 新增 `core/knowledge_store.rs`，使用 `rusqlite` + bounded channel + WAL
  - Knowledge Store write path 支援 action/clipboard batch insert；shutdown 前先 flush DB worker，避免 pending writes 遺失
  - 新增 Agent 資料模型、Agent mode UI、`docs/ai_agent.md`、`docs/ai_context_privacy.md`
  - 新增 Plugin manifest / permission / audit model、plugin.toml/json loader 與 `docs/plugin_security.md`
  - `SettingPanel` 改由 schema 驅動，敏感設定遮蔽顯示

## ADR-015 Search Backend Preference 與重建入口

- 狀態：Accepted
- 決策：搜尋後端以 `[search].backend` 設定表達偏好（`auto` / `everything` / `app_cache` / `tantivy`），由 `SearchManager::select_backend` 依可用性決定 active backend；`search.backend` 回傳 structured debug info，`/rebuild_search_index` 先重建目前 Windows file cache，未來 Tantivy provider 接入後沿用同一入口。
- 原因：
  - `auto` 讓 Windows 使用者可優先走 Everything，缺少 DLL/服務時仍有 app/file cache fallback
  - backend selection 純函式化後可用 `test_backend_selection_*` 鎖住回退規則
  - 重建入口先穩定 UI/IPC/command contract，避免之後接 Tantivy 時再改前端命令名稱
- 影響：
  - `managers/search_manager.rs` 新增 `SearchBackendPreference`、`SearchBackendInfo`、rebuild status 與 selection tests
  - `handlers/search.rs` 新增 `search.rebuild_index`，`handlers/builtin_cmd.rs` 新增 `/rebuild_search_index`
  - `default_config.toml` 與 settings schema 新增 `[search]` keys
  - `CommandPalette` 顯示 search backend debug badge

## ADR-016 Agent Read-only Tool Runtime

- 狀態：Accepted
- 決策：`/ai` Agent 第一階段只執行 read-only tools。`agent.tool` 支援 `keynova.search` 與 `web.search`；`agent.start` 只在提示詞需要本機搜尋時帶入 `keynova.search` grounding sources，不執行本機寫入或 shell。`web.search` 第一版只支援 SearXNG JSON provider，預設 disabled。
- 原因：
  - 先把 tool boundary、visibility filter、redaction 與 source ViewModel 做實，再做 approval/action phases
  - `keynova.search` 可安全讀取 workspace summary、command metadata、非敏感 settings schema、model catalog、notes/history 摘要
  - `web.search` 不抓完整頁面，只回 title/url/snippet，並在送出前拒絕 `private_architecture` / `secret` query term
- 影響：
  - `handlers/agent.rs` 注入 config/note/history/workspace/command/model context，新增 `agent.tool`
  - `default_config.toml` 與 schema 新增 `agent.web_search_provider`、`agent.searxng_url`、`agent.web_search_api_key`、`agent.web_search_timeout_secs`
  - `AgentRuntime` 新增 `start_with_context`，讓 run output 帶 grounding source 摘要
  - 搜尋 backend 新增 generation/cancel，前端離開搜尋時呼叫 `search.cancel`，降低 stale provider result 覆蓋新查詢的機率

## 待補強事項

- Everything IPC 實作目前仍屬規劃，需補官方 SDK 細節與錯誤處理策略
- 後續新增 ADR 時，請依 `ADR-00X` 連號並補上影響範圍
