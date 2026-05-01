# tasks.md — 任務追蹤

> 完成任務請打 `[x]`，並在 memory.md 更新「上次完成」與「下一步」。
> 阻塞中的任務請移至「阻塞中」區塊並說明原因。

---

## Phase 0 — 專案初始化（當前）

### 本週任務

- [x] 初始化 Tauri 專案（`npm create tauri-app@latest`）
- [x] 設定 ESLint + Prettier（`.eslintrc.cjs`、`prettier.config.cjs`）
- [x] 設定 Tailwind CSS
- [x] 建立 `src-tauri/src/core/` 目錄結構
- [x] 建立 `CommandRouter` trait 骨架（Rust）
- [x] 建立 `EventBus` 骨架（Rust broadcast channel）
- [x] 確認 Tauri IPC 基本通訊可運作（前端呼叫 → Rust handler 回傳）

### 已完成

- [x] 撰寫 `files/` 下的所有規格文件
- [x] 建立 `CLAUDE.md`
- [x] 建立 `memory.md`、`decisions.md`、`tasks.md`
- [x] 建立 `skill.md`（Codex 能力規範）
- [x] 建立 `AGENTS.md`（指向 CLAUDE.md）
- [x] 加入 Git 工作流程規範（CLAUDE.md）
- [x] 加入 Session 收尾協議：任務完成後自動更新三份文件（CLAUDE.md / skill.md）

---

## Phase 1 — MVP v0.1（weeks 1–6）

- [ ] App Launcher（Win+K）：模糊搜尋 + 啟動應用程式
- [ ] 浮動終端視窗（Ctrl+Alt+T）：PTY + xterm.js + 分頁
- [ ] 鍵盤滑鼠控制（WASD/方向鍵）：全鍵盤操控游標

---

## Phase 2 — v1.0（weeks 7–12）

### 檔案搜尋（Ctrl+P）— 拆分為兩個子版本

**v1.0：Tantivy 核心實作**
- [ ] `SearchResult` 模型（含 `backend` 字段）
- [ ] `SearchManager` 骨架 + `SearchBackend` 枚舉
- [ ] Tantivy 索引建立 + `search_with_tantivy()`
- [ ] RPC 命令：`cmd_search`、`cmd_rebuild_search_index`
- [ ] 前端 `useSearch` hook（含 loading 狀態）
- [ ] `default_config.toml` 加入 `[search]` 區塊

**v1.5：Everything IPC 整合（Windows）**
- [ ] `src-tauri/src/utils/everything_ipc.rs`：`check_availability()` + `search()` 實作（Windows IPC）
- [ ] `SearchManager::detect_everything_service()`（100ms 逾時偵測）
- [ ] `SearchManager::select_backend()` 自動選擇邏輯
- [ ] `search_with_everything()` + `SearchStats` 統計
- [ ] RPC 命令：`cmd_get_search_backend`、`cmd_get_search_stats`
- [ ] 前端顯示當前後端（`activeBackend`）
- [ ] 單元測試：`test_backend_selection_*` 三個情境

### 其他 Phase 2 功能
- [ ] 計算機（Ctrl+=）：單位換算、進位換算
- [ ] 剪貼簿歷史（Ctrl+Shift+V）：圖片預覽、歷史管理
- [ ] 系統控制（Win+S）：音量、亮度、WiFi

---

## Phase 3 — v2.0（weeks 13–18）

- [ ] AI 助理（Ctrl+Shift+A）：Claude API 整合
- [ ] 虛擬工作區（Ctrl+Alt+1/2/3）
- [ ] 快速筆記（Win+N）：Markdown + Notion 同步

---

## Phase 4 — v3.0+

- [ ] Plugin System（JS/Python 擴展）
- [ ] 雲端同步
- [ ] 公開 API

---

## 阻塞中

- 無
