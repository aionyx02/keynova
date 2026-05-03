# memory.md — AI 工作記憶

> 每次開新 session，AI 必須先讀此檔案，確認現在進度，再繼續工作。
> 完成任何重要步驟後，請更新此檔案。

## 當前狀態

- **進度**：Phase 2.A 完成 + Task 1（全域搜尋）完成 + Phase 2.B 實作完成（feature/phase2 分支）
- **上次完成**：Phase 2.B 全部實作：BuiltinCommandRegistry trait + HelpCommand/SettingCommand + BuiltinCmdHandler + SettingHandler + ConfigManager TOML I/O + useCommands + CommandSuggestions + SettingPanel + PanelRegistry + CommandPalette command 模式接線；cargo clippy + npm lint 全通過
- **下一步**：commit Phase 2.B 成果 → 等待使用者審查並決定是否合併 dev；接著可繼續 Phase 2 剩餘功能（計算機、剪貼簿歷史、Tantivy 離線索引）

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

## 歷史摘要（已壓縮）

- 規劃 → Phase 0 → Phase 1 完成（2026-05-01 ~ 05-02）：文件建立、Tauri scaffold、ESLint/Tailwind、CommandRouter/EventBus、handlers/managers/models/platform 骨架、Flow Launcher 風格 UI、全域滑鼠控制、Everything 搜尋架構、51 files 合併進 main
- Phase 2 規劃（2026-05-02）：tasks 設計完成；Phase 2.A 搜尋框三模式 + Phase 2.B /command Registry；feature/phase2 分支建立

## Session 交接紀錄（最近 5 筆）

| 日期 | 完成事項 | 遺留問題 |
|------|----------|----------|
| 2026-05-03 | Phase 2.B + Task 1（D槽/WSL全域搜尋）全部驗收通過；BuiltinCommandRegistry / HelpCommand / SettingCommand / BuiltinCmdHandler / SettingHandler / ConfigManager TOML I/O / 前端 useCommands + CommandSuggestions + SettingPanel + PanelRegistry + CommandPalette command 模式 | — |
| 2026-05-03 | 規劃 Task 1（全域搜尋擴展）+ Phase 2.B 任務細化，寫入 tasks.md / memory.md；未開始實作 | — |
| 2026-05-03 | Phase 2.A terminal fix: PowerShell/pwsh default shell, TERM/WSLENV hints, real PTY resize, terminal-output event, ESC focus guard | Verified with `npm run tauri dev`: `>` opens prompt, `clear` works, WSL cursor stays aligned, ESC returns to 60px Search UI; lint/tsc/clippy passed |
| 2026-05-03 | Phase 2.A 架構精化：Terminal 模式改為 xterm.js + portable-pty 真實 PTY；Phase 2.B 加入 PanelRegistry 擴充點；tasks.md 全面重寫 2.A / 2.B | — |
| 2026-05-03 | Phase 2.A 架構精化：Terminal 模式改為 xterm.js + portable-pty 真實 PTY；Phase 2.B 加入 PanelRegistry 擴充點；tasks.md 全面重寫 2.A / 2.B | — |
| 2026-05-03 | Phase 2.A 實作：useInputMode / useTerminalTheme / TerminalPanel（xterm.js + PTY IPC）/ CommandPalette 三模式 / EventBus→Tauri emit 橋接；feature/phase2 建立；cargo clippy + tsc + lint 通過 | 需 `npm run tauri dev` 驗收：輸入 `>` 展開終端、互動指令、ESC 收回 |
| 2026-05-03 | 終端機三 UX 修復：(1) PTY pre-warm（app 啟動時預建 cmd.exe，消除 >1 秒延遲）(2) xterm.focus() 自動聚焦 (3) attachCustomKeyEventHandler 實作 ESC/Ctrl+C 退出並回到搜尋框；ADR-008 新增；cargo clippy + lint 全通過 | 需 `npm run tauri dev` 手動驗收三個修復 |

## 2026-05-03 架構邊界與修正定位索引

目的：未來修正時，直接定位「單一模組/檔案」即可，不需掃描全域程式碼。

### A. 模組邊界（Boundary）

1. 前端 UI 與互動層（React/Tauri API）
   - `/src/main.tsx`：前端入口，掛載 `<App />`
   - `/src/App.tsx`：根組件，目前以 `CommandPalette` 為主畫面
   - `/src/components/CommandPalette.tsx`：主互動面板（搜尋/命令/終端切換、結果選取、視窗焦點行為）
   - `/src/components/TerminalPanel.tsx`：xterm UI 與 terminal IPC（open/send/resize/close）
   - `/src/hooks/useIPC.ts`：統一 IPC 入口，所有前端命令透過 `cmd_dispatch`
   - `/src/stores/appStore.ts`：全域狀態（query/searchResults/loading）

2. 後端命令分派層（Tauri + Rust）
   - `/src-tauri/src/lib.rs`：AppState 建立、handler 註冊、事件橋接、快捷鍵與視窗生命週期
   - `/src-tauri/src/core/command_router.rs`：`namespace.command` 路由分派核心
   - `/src-tauri/src/core/event_bus.rs`：後端事件匯流排（broadcast）

3. 後端功能 Handler 層（命令入口）
   - `/src-tauri/src/handlers/launcher.rs`：`launcher.*` 命令入口
   - `/src-tauri/src/handlers/hotkey.rs`：`hotkey.*` 命令入口
   - `/src-tauri/src/handlers/terminal.rs`：`terminal.*` 命令入口
   - `/src-tauri/src/handlers/mouse.rs`：`mouse.*` 命令入口
   - `/src-tauri/src/handlers/search.rs`：`search.*` 命令入口

4. 後端 Manager 層（業務邏輯）
   - `/src-tauri/src/managers/app_manager.rs`：App 掃描、快取、模糊搜尋、啟動
   - `/src-tauri/src/managers/hotkey_manager.rs`：熱鍵註冊資料與衝突檢查
   - `/src-tauri/src/managers/terminal_manager.rs`：PTY session 管理與 pre-warm
   - `/src-tauri/src/managers/mouse_manager.rs`：滑鼠/鍵盤模擬行為封裝
   - `/src-tauri/src/managers/search_manager.rs`：App + File/Folder 搜尋聚合（Everything / fallback）

5. 平台實作層（Platform）
   - `/src-tauri/src/platform/windows.rs`：Windows 實作（App 掃描、輸入模擬、Everything IPC、檔案 fallback 掃描）
   - `/src-tauri/src/platform/linux.rs`：Linux 預留（尚未完整實作）
   - `/src-tauri/src/platform/macos.rs`：macOS 預留（尚未完整實作）

6. 設定與權限邊界
   - `/src-tauri/tauri.conf.json`：視窗行為、dev/build 路徑、bundle
   - `/src-tauri/capabilities/default.json`：Tauri API 權限白名單

### B. 主要呼叫路徑（Execution Path）

1. 前端命令流
   - `/src/components/CommandPalette.tsx`
   -> `/src/hooks/useIPC.ts`
   -> Tauri command `cmd_dispatch`
   -> `/src-tauri/src/core/command_router.rs`
   -> `/src-tauri/src/handlers/*.rs`
   -> `/src-tauri/src/managers/*.rs`
   -> `/src-tauri/src/platform/*.rs`（視需要）

2. 後端事件回推前端
   - `/src-tauri/src/managers/terminal_manager.rs` 產生輸出
   -> `/src-tauri/src/core/event_bus.rs` publish
   -> `/src-tauri/src/lib.rs` setup bridge（topic 轉 kebab-case）
   -> 前端 `listen("terminal-output")`（`/src/components/TerminalPanel.tsx`）

### C. 修正定位清單（Issue -> File）

1. 搜尋結果不正確 / 來源異常
   - 先看：`/src-tauri/src/managers/search_manager.rs`
   - Windows 搜尋來源：`/src-tauri/src/platform/windows.rs`
   - 前端顯示與鍵盤選取：`/src/components/CommandPalette.tsx`

2. App 啟動失敗 / App 清單不完整
   - 啟動與掃描邏輯：`/src-tauri/src/managers/app_manager.rs`
   - Windows 掃描與啟動：`/src-tauri/src/platform/windows.rs`
   - 命令入口：`/src-tauri/src/handlers/launcher.rs`

3. Terminal 首開慢 / 無輸出 / resize 問題
   - PTY 與 pre-warm：`/src-tauri/src/managers/terminal_manager.rs`
   - terminal IPC 命令：`/src-tauri/src/handlers/terminal.rs`
   - xterm 綁定與事件：`/src/components/TerminalPanel.tsx`

4. 快捷鍵衝突 / 無法觸發
   - 註冊流程與衝突檢查：`/src-tauri/src/managers/hotkey_manager.rs`
   - 命令入口：`/src-tauri/src/handlers/hotkey.rs`
   - 全域 shortcut setup：`/src-tauri/src/lib.rs`

5. 滑鼠鍵盤模擬異常（W/A/S/D、點擊、輸入）
   - 業務封裝：`/src-tauri/src/managers/mouse_manager.rs`
   - 命令入口：`/src-tauri/src/handlers/mouse.rs`
   - Windows API 實作：`/src-tauri/src/platform/windows.rs`

6. 視窗焦點/隱藏行為異常（Esc、blur、show/hide）
   - 視窗事件與 hide/show：`/src-tauri/src/lib.rs`
   - 前端 focus 流與 Esc 行為：`/src/components/CommandPalette.tsx`
   - Tauri 視窗 API 權限：`/src-tauri/capabilities/default.json`

### D. 快速搜尋關鍵字

- `terminal-output`：終端事件回推
- `cmd_dispatch`：前後端命令橋接入口
- `namespace.command`：路由規範
- `prewarm`：終端預熱流程
- `Everything`：Windows 檔案搜尋來源