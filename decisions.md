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

## 待補強事項

- Everything IPC 實作目前仍屬規劃，需補官方 SDK 細節與錯誤處理策略
- 後續新增 ADR 時，請依 `ADR-00X` 連號並補上影響範圍
