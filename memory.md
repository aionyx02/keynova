# memory.md — AI 工作記憶

> 每次開新 session，AI 必須先讀此檔案，確認現在進度，再繼續工作。
> 完成任何重要步驟後，請更新此檔案。

## 當前狀態

- **進度**：Phase 1 骨架實作完成（feature/phase1-setup 分支，等待使用者審查後合併 dev）
- **上次完成**：修復 `cmd_ping` 因 EventBus 無訂閱者回傳 `channel closed` 的問題，確認 `cargo check`、`npm run lint`、`npm run build` 通過
- **下一步**：使用者審查 diff → 確認合併 feature/phase1-setup 到 dev → 實作 1.2 熱鍵 WinAPI RegisterHotKey + message loop（目前為 TODO）

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

## Session 交接紀錄

| 日期 | 完成事項 | 遺留問題 |
|------|----------|----------|
| 2026-05-01 | 建立文件骨架（CLAUDE.md / memory.md / decisions.md / tasks.md）| 尚未初始化 Tauri 專案 |
| 2026-05-01 | 完成 Everything 雙後端搜尋架構設計，更新 decisions.md（ADR-006）、tasks.md（Phase 2 子任務）、memory.md | everything_ipc.rs 實作為佔位符，需實際 IPC 開發 |
| 2026-05-01 | 初始化 Tauri 專案（React-TS 模板），並恢復被 scaffold 覆蓋的根目錄規範文件 | 目前模板為 React 19，需評估是否鎖回 React 18 以符合規劃 |
| 2026-05-01 | 建立 skill.md / AGENTS.md，加入 Git 工作流程規範與 Session 收尾協議 | 無 |
| 2026-05-01 | 更新 .gitignore（JetBrains RustRover），CLAUDE.md 加開發環境說明，decisions.md 加 ADR-007 | 無 |
| 2026-05-01 | 完成 Phase 0：ESLint/Prettier、Tailwind、core 架構骨架、CommandRouter、EventBus、前後端 IPC ping 串接 | 本機缺少 cargo 指令，尚未執行 Rust 編譯檢查 |
| 2026-05-01 | 細化 tasks.md Phase 1（1.0 依賴、1.1 App Launcher、1.2 全局熱鍵、1.3 浮動終端、1.4 鍵盤滑鼠控制，含驗收標準） | 無 |
| 2026-05-01 | Phase 1 骨架實作：全部 handlers/managers/models/platform/前端組件，feature/phase1-setup 分支，ESLint 通過，等待審查 | 1.2 熱鍵 WinAPI message loop 尚為 TODO placeholder；resize_pty 需 retain master PTY 後再補齊 |
| 2026-05-01 | 修復 LauncherHandler `list_all` 可變借用，並通過 `cargo check`、`npm run lint`、`npm run build` | 目前 shell 仍需先把 `C:\Users\shawn\.cargo\bin` 加入 PATH，重開終端後應可直接使用 cargo |
| 2026-05-01 | 修復 `cmd_ping` 將 EventBus 無訂閱者視為 IPC 失敗，避免畫面顯示 `IPC call failed: channel closed` | 無 |
