# memory.md — AI 工作記憶

> 每次開新 session，AI 必須先讀此檔案，確認現在進度，再繼續工作。
> 完成任何重要步驟後，請更新此檔案。

## 當前狀態

- **進度**：Phase 2 全部完成 + BUG-1~10 + FEAT-1~5（feature/feat1-feat2-control-plane 分支）
- **上次完成**：FEAT-3（指令參數提示）、FEAT-4（終端內容保留）、FEAT-5（設定表格鍵盤導航）全部實作完成，tsc/lint/clippy 全綠
- **下一步**：手動驗收 FEAT-3~5；驗收通過後合併 feature/feat1-feat2-control-plane → dev → main

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

## Session 交接紀錄（最近 5 筆）

| 日期 | 完成事項 | 遺留問題 |
|------|----------|----------|
| 2026-05-04 | FEAT-3（指令參數提示：CommandMeta args_hint、cmd.suggest_args、hint bar + arg suggestions 下拉）、FEAT-4（終端內容保留：terminalMounted + CSS display、TerminalPanel isActive prop）、FEAT-5（設定鍵盤導航：inputRefs、auto-focus、↑↓←→ + Enter）；tsc/lint/clippy 全綠 | 需手動驗收三個功能 |
| 2026-05-03 | FEAT-2 完成：`keynova start/down/reload/status` CLI、loopback control plane、config reload snapshot/diff、hotkey re-register、notify watcher、`/reload` `/down`；外部存檔 watcher 自動套用 | 未 push 遠端 |
| 2026-05-03 | BUG-10 儲存 flash + 生效時機 hint；FEAT-1 /setting key value Minecraft 格式；BuiltinCmdHandler 注入 ConfigManager；tsc/lint/clippy 全綠 | 需手動驗收 BUG-10、FEAT-1 |
| 2026-05-03 | BUG-6~9 全部實作完成（漸進式 ESC、Tab 補全、config 生效、熱鍵捕捉）；tsc/lint/clippy 全綠 | — |
| 2026-05-03 | private docs 因 git merge 誤刪，已從 git 歷史還原；BUG-6~9 加入 tasks.md；feature/bugfix-ux 建立 | — |

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
   - `/src-tauri/src/platform/windows.rs`：Windows 實作（App 掃描、輸入模擬、Everything IPC、檔案 cache 索引）
   - `/src-tauri/src/platform/linux.rs`：Linux 預留（尚未完整實作）
   - `/src-tauri/src/platform/macos.rs`：macOS 預留（尚未完整實作）

6. 設定與權限邊界
   - `/src-tauri/tauri.conf.json`：視窗行為、dev/build 路徑、bundle、CSP
   - `/src-tauri/capabilities/default.json`：Tauri API 權限白名單

### B. 快速搜尋關鍵字

- `terminal-output`：終端事件回推
- `cmd_dispatch`：前後端命令橋接入口
- `namespace.command`：路由規範
- `prewarm`：終端預熱流程
- `Everything`：Windows 檔案搜尋來源
- `FILE_CACHE`：Windows 背景檔案索引
