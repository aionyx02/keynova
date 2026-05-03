# decisions.md — 架構決策紀錄（ADR）

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
- 決策：以 `toml` crate 讀寫 `%APPDATA%\Keynova\config.toml`；內部以 flat dot-key HashMap 表示（如 `hotkeys.app_launcher`），讀取時遞迴攤平 TOML Table，寫回時以 dotted key 格式序列化
- 原因：
  - 避免引入 JSON 之外的額外序列化格式；TOML 與既有 `default_config.toml` 格式一致
  - Flat HashMap 使 `setting.get/set` IPC 介面簡單，鍵名可直接作為 UI label
  - 使用者設定優先；缺少時 fallback 至 `default_config.toml`（套件內）
- 影響：
  - `core/config_manager.rs` 完全取代先前的 stub
  - `handlers/setting.rs` 新增，提供 `setting.get/set/list_all` IPC
  - 前端 `SettingPanel` 分四 tab 呈現，onChange 直接呼叫 `setting.set`

## 待補強事項

- Everything IPC 實作目前仍屬規劃，需補官方 SDK 細節與錯誤處理策略
- 後續新增 ADR 時，請依 `ADR-00X` 連號並補上影響範圍
