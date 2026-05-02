# memory.md — AI 工作記憶

> 每次開新 session，AI 必須先讀此檔案，確認現在進度，再繼續工作。
> 完成任何重要步驟後，請更新此檔案。

## 當前狀態

- **進度**：Phase 1.4–1.6 完成，Bug Fix：卡鍵修正 + 資料夾搜尋 + Flow Launcher 視窗，cargo check + clippy + lint + tsc 全通過
- **上次完成**：Phase 1 所有任務完成並驗收，feature/phase1-setup → dev → main 兩段 --no-ff 合併，目前在 main 分支
- **下一步**：從 dev 建立 feature/phase2 分支，開始 Phase 2（計算機、剪貼簿、系統控制、Tantivy 離線索引）

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

- 規劃與文件設計（2026-05-01）：建立 CLAUDE.md / memory.md / decisions.md / tasks.md / skill.md / AGENTS.md；完成 Everything 雙後端搜尋架構設計（ADR-006）
- 專案初始化（2026-05-01）：Tauri scaffold（React-TS 模板）、.gitignore / ADR-007 設定、開發環境說明補充
- Phase 0 完成（2026-05-01）：ESLint / Prettier / Tailwind / CommandRouter / EventBus / 前後端 IPC ping 串接；tasks.md Phase 1 全部驗收標準細化
- Phase 1 骨架與 UI 精化（2026-05-01）：handlers / managers / models / platform / 前端組件全部骨架建立；早期修復；App.tsx Keynova 入口整合；Phase 1.5 Flow Launcher 風格 UI（transparent 視窗、全局 Ctrl+K、系統托盤、CommandPalette 重設計）

## Session 交接紀錄（最近 5 筆）

| 日期 | 完成事項 | 遺留問題 |
|------|----------|----------|
| 2026-05-02 | 全域 Alt+W/A/S/D 滑鼠控制（Rust 全域熱鍵，Alt+M 切換）、UI 僅剩搜尋框（移除 MouseControlOverlay）、提前完成搜尋架構（Everything DLL IPC + SearchManager + search.query + pre-scan）、全部 cargo check + lint + tsc 通過 | 驗收需 `npm run tauri dev` 確認：透明視窗、無冷啟動延遲、Everything（若安裝）、Alt 系列熱鍵 |
| 2026-05-02 | Bug Fix：(1) Alt→Ctrl+Alt 防卡鍵，(2) Everything IsFolderResult + ResultKind::Folder，(3) 視窗改為 640×60 搜尋框大小、動態 resize、失焦自動隱藏；cargo check + clippy + lint + tsc 全通過 | 驗收需 `npm run tauri dev`：Ctrl+K 小視窗 + 展開 + 資料夾搜尋 + Ctrl+Alt+M |
| 2026-05-02 | README.md 改寫為 Keynova 正式文件（快捷鍵速查表，已實作/規劃分組）；tasks.md 加入「快捷鍵文件同步」持續維護規則；CLAUDE.md + skill.md 加入記憶壓縮協議並執行首次壓縮 | 無 |
| 2026-05-02 | 修復搜尋只找 App 問題：加入 scan_files_basic()（Desktop/Downloads/Documents，深度 3），AppCache fallback 自動呼叫；clippy 通過 | 需 tauri dev 驗收：搜尋結果應出現檔案/資料夾 |
| 2026-05-02 | Phase 1 全部驗收完成；tasks.md 全打 [x]；feature/phase1-setup → dev → main 兩段 --no-ff 合併；51 files，2799 insertions | 無 |
