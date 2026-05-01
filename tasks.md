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

### 1.0 前置依賴與目錄結構（Week 1）

- [x] 安裝前端依賴：`zustand`（狀態管理）
- [x] 安裝前端依賴：`@xterm/xterm`、`@xterm/addon-fit`（終端模擬，已改用官方新套件）
- [x] `Cargo.toml` 新增：`portable-pty`（PTY 跨平台支援）
- [x] `Cargo.toml` 新增：`windows`（Windows API 整合，feature: Win32_UI_Input_KeyboardAndMouse、Win32_System_Threading）
- [x] 建立前端目錄：`src/types/`、`src/hooks/`、`src/stores/`、`src/services/`
- [x] 建立後端目錄：`src-tauri/src/handlers/`、`src-tauri/src/managers/`、`src-tauri/src/models/`、`src-tauri/src/platform/`
- [x] 建立 `src-tauri/default_config.toml`（預設熱鍵：Win+K, Ctrl+Alt+T）

---

### 1.1 App Launcher（Win+K）— Weeks 2–3

**模型層**
- [x] `src-tauri/src/models/app.rs`：AppInfo 結構體（name, path, icon_data, launch_count）

**管理層**
- [x] `src-tauri/src/managers/app_manager.rs`：AppManager
  - `scan_applications() -> Vec<AppInfo>` — 掃描 Start Menu + AppData\Local\Programs
  - `search_apps(query: &str) -> Vec<AppInfo>` — 模糊搜尋（含記憶體快取）
  - `launch_app(path: &str) -> Result<()>` — 以路徑啟動應用程式

**處理器層**
- [x] `src-tauri/src/handlers/launcher.rs`：LauncherHandler（impl CommandHandler）
  - 命令：`launcher.search`、`launcher.launch`、`launcher.list_all`
- [x] 在 `lib.rs` 向 CommandRouter 註冊 LauncherHandler

**平台層**
- [x] `src-tauri/src/platform/windows.rs`：應用掃描（`%APPDATA%\...\Start Menu`、`%LOCALAPPDATA%\Programs`、`.lnk` 解析）

**前端**
- [x] `src/types/app.ts`：AppInfo TypeScript 型別
- [x] `src/hooks/useIPC.ts`：通用 Tauri invoke 包裝 Hook（含泛型型別）
- [x] `src/hooks/useEvents.ts`：EventBus 監聽 Hook（含 unlisten 清理）
- [x] `src/stores/appStore.ts`：應用列表與搜尋結果 Zustand store
- [x] `src/components/CommandPalette.tsx`：命令面板（搜尋框 + 結果列表 + ↑↓Enter 鍵盤導航）
- [x] `src/components/FloatingWindow.tsx`：浮動視窗容器（可移動、可調大小）

**驗收**
- [ ] Win+K 開啟命令面板，1 秒內顯示應用列表
- [ ] 模糊搜尋即時更新（<100ms）
- [ ] Enter 鍵啟動所選應用程式

---

### 1.2 全局熱鍵系統（Weeks 2–4）

**模型層**
- [x] `src-tauri/src/models/hotkey.rs`：HotkeyConfig 結構體（id, key, modifiers, action, enabled）

**管理層**
- [x] `src-tauri/src/managers/hotkey_manager.rs`：HotkeyManager
  - `register_global_hotkey(config: &HotkeyConfig) -> Result<()>`
  - `unregister_hotkey(id: &str) -> Result<()>`
  - `check_conflict(config: &HotkeyConfig) -> Option<ConflictInfo>`
  - `list_hotkeys() -> Vec<HotkeyConfig>`

**處理器層**
- [x] `src-tauri/src/handlers/hotkey.rs`：HotkeyHandler（impl CommandHandler）
  - 命令：`hotkey.register`、`hotkey.unregister`、`hotkey.list`、`hotkey.check_conflict`
- [x] 在 `lib.rs` 向 CommandRouter 註冊 HotkeyHandler

**平台層**
- [ ] `src-tauri/src/platform/windows.rs`：`RegisterHotKey` / `UnregisterHotKey` WinAPI + message loop 整合（骨架已建立，實際 WinAPI 呼叫待實作）

**前端**
- [x] `src/types/hotkey.ts`：HotkeyConfig TypeScript 型別
- [x] `src/hooks/useHotkey.ts`：熱鍵管理 Hook（register / unregister / list）

**驗收**
- [ ] Win+K、Ctrl+Alt+T 在所有前景應用下均可觸發
- [ ] 衝突偵測：重複或系統衝突時回傳可讀錯誤訊息

---

### 1.3 浮動終端視窗（Ctrl+Alt+T）— Weeks 3–5

**模型層**
- [x] `src-tauri/src/models/terminal.rs`：TerminalSession 結構體（id, rows, cols, status）

**管理層**
- [x] `src-tauri/src/managers/terminal_manager.rs`：TerminalManager
  - `create_pty() -> Result<String>` — 建立 PTY，回傳 terminal_id
  - `write_to_pty(id: &str, input: &str) -> Result<()>`
  - `read_from_pty_loop(id: &str)` → 透過 EventBus 推送 `terminal.output`
  - `resize_pty(id: &str, rows: u16, cols: u16) -> Result<()>`
  - `close_pty(id: &str) -> Result<()>`

**處理器層**
- [x] `src-tauri/src/handlers/terminal.rs`：TerminalHandler（impl CommandHandler）
  - 命令：`terminal.open`、`terminal.send`、`terminal.close`、`terminal.resize`
- [x] 在 `lib.rs` 向 CommandRouter 註冊 TerminalHandler

**前端**
- [x] `src/types/terminal.ts`：TerminalSession TypeScript 型別
- [x] `src/hooks/useTerminal.ts`：終端 Hook（open/send/close + 監聽 `terminal.output` 事件 + unlisten）
- [x] `src/components/TerminalView.tsx`：xterm.js 包裝（ANSI 顏色、resize 事件、多分頁 TabBar）

**驗收**
- [ ] Ctrl+Alt+T 開啟浮動終端視窗
- [ ] Shell 命令可執行並即時顯示輸出（無明顯延遲）
- [ ] 支援同時開啟多個終端分頁

---

### 1.4 鍵盤滑鼠控制（Weeks 5–6）

**管理層**
- [x] `src-tauri/src/managers/mouse_manager.rs`：MouseManager
  - `move_cursor_relative(dx: i32, dy: i32) -> Result<()>` — WASD/方向鍵相對移動
  - `move_cursor_absolute(x: i32, y: i32) -> Result<()>`
  - `simulate_click(button: &str, count: u32) -> Result<()>`（left/right/middle）
  - `simulate_key(key: &str, modifiers: &[String]) -> Result<()>`
  - `type_text(text: &str) -> Result<()>`

**處理器層**
- [x] `src-tauri/src/handlers/mouse.rs`：MouseHandler（impl CommandHandler）
  - 命令：`mouse.move_relative`、`mouse.move_absolute`、`mouse.click`、`mouse.type`、`mouse.press_key`
- [x] 在 `lib.rs` 向 CommandRouter 註冊 MouseHandler

**平台層**
- [x] `src-tauri/src/platform/windows.rs`：`SendInput` WinAPI 整合（MOUSEINPUT + KEYBDINPUT）

**前端**
- [x] `src/components/MouseControlOverlay.tsx`：控制模式啟用/關閉狀態列指示器
- [x] `src/hooks/useMouseControl.ts`：鍵盤滑鼠控制 Hook（模式切換 + 按鍵映射）

**驗收**
- [ ] WASD / 方向鍵可控制游標移動（可調整步進速度）
- [ ] Enter / Space 執行滑鼠左鍵點擊
- [ ] 切換控制模式時有明確視覺提示

---

### Phase 1 端到端驗收標準

- [ ] App Launcher：Win+K → 搜尋 → Enter 開啟應用（冷啟動 <100ms）
- [ ] 全局熱鍵：Win+K、Ctrl+Alt+T 在所有前景視窗下均可觸發
- [ ] 浮動終端：Ctrl+Alt+T → 執行命令 → 即時輸出
- [ ] 鍵盤滑鼠：方向鍵游標移動 + Enter 點擊端到端整合
- [ ] 跨平台：Windows 10+ 所有功能手動測試通過
- [ ] 品質：`cargo clippy -- -D warnings` 零 warning；`npm run lint` 零錯誤

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
