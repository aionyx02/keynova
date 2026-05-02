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

**平台層（全域熱鍵，Rust 端）**
- [x] `lib.rs`：`setup_global_shortcuts` 中直接以 `tauri-plugin-global-shortcut` 全域註冊
  - `Alt+M`：切換滑鼠控制模式，發出 `mouse-control-toggled` 事件
  - `Alt+W/A/S/D`：模式啟用時移動游標（步進 15px）
  - `Alt+Return`：模式啟用時模擬左鍵點擊
  - `AppState.mouse_active`：`Arc<AtomicBool>` 跨 handler 共享模式狀態

**前端**
- [x] `src/components/MouseControlOverlay.tsx`：監聽 `mouse-control-toggled` 事件，只顯示狀態（不再掛載至 App.tsx）
- [x] `src/hooks/useMouseControl.ts`：改為監聽 Tauri 事件，移除 keydown listener

**驗收**
- [ ] Alt+W/A/S/D 在任意前景應用下均可移動游標（全域，無需 Keynova 視窗 focus）
- [ ] Alt+Return 執行滑鼠左鍵點擊（全域）
- [ ] Alt+M 切換模式，切換後 mouse-control-toggled 事件正確傳遞至前端

---

### 1.5 CommandPalette UI 精化（Flow Launcher 風格）

> 參考：[Flow Launcher](https://www.flowlauncher.com/) — 居中、無邊框、背景常駐、輸入後才展開結果

**Tauri 視窗設定**
- [x] `tauri.conf.json`：主視窗設定 `transparent: true`、`decorations: false`（無邊框透明視窗）
- [x] `tauri.conf.json`：主視窗預設 `visible: false`（啟動後立即隱藏，等待熱鍵觸發）
- [x] 實作系統托盤（System Tray）：`tauri` tray-icon feature，右鍵選單含「顯示」「退出」
- [x] `tauri-plugin-global-shortcut`：背景常駐全局 Ctrl+K 熱鍵（Rust 端註冊）
- [x] `on_window_event` CloseRequested → `prevent_close` + `hide`（視窗不真正關閉）
- [x] `window-focused` Tauri event：視窗重新出現時通知前端清空並自動 focus

**CommandPalette 重設計**
- [x] 全螢幕透明覆蓋層：`position: fixed; inset: 0`，點擊遮罩呼叫 `hideWindow()`
- [x] 搜尋框固定於螢幕正中央偏上（`paddingTop: 28vh`），寬度約 640px
- [x] 無搜尋時：只顯示單行輸入框，無結果區、無外框
- [x] 輸入後：結果清單從輸入框下方向下展開（`rounded-b-xl` 承接上框）
- [x] 輸入框樣式：無邊框，深色半透明背景（`bg-gray-900/92`）+ `backdrop-blur-md`，圓角

**行為邏輯**
- [x] Ctrl+K 觸發：Rust `setup_global_shortcut` 呼叫 `window.show()` + `set_focus()`
- [x] Escape 或點擊遮罩：呼叫 `getCurrentWindow().hide()`（App 常駐背景）
- [x] 成功啟動應用程式後自動呼叫 `hideWindow()`
- [x] 新增 Tauri command：`cmd_show_launcher`、`cmd_hide_launcher`
- [x] `App.tsx` 簡化：移除 `paletteOpen` 狀態，CommandPalette 常駐，視窗可見性即 UI 可見性

**驗收**
- [ ] 按 Ctrl+K 後出現居中輸入框，無任何視窗邊框與背景色塊
- [ ] 未輸入時不顯示結果區
- [ ] 輸入後結果從框下方滑出展開，風格類似 Flow Launcher
- [ ] Escape 關閉但 App 仍在背景執行（系統托盤可見）

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

### 搜尋系統（提前至 Phase 1.6 完成）

> 已提前完成核心搜尋架構與 Everything IPC

**已完成（提前至 Phase 1）**
- [x] `src-tauri/src/models/search_result.rs`：`SearchResult` 模型（kind: App/File, name, path, score）
- [x] `src-tauri/src/managers/search_manager.rs`：`SearchManager` + `SearchBackend` 枚舉（Everything / AppCache）
  - `detect_backend()` — 啟動時偵測 Everything64.dll 是否可載入
  - `search(query, limit)` — App 快取模糊搜尋 + Everything 檔案搜尋並合併結果
- [x] `src-tauri/src/platform/windows.rs`：`check_everything()` + `everything_search()` — DLL 動態載入（LibraryLoader API）
- [x] `src-tauri/src/handlers/search.rs`：`SearchHandler`（`search.query` / `search.backend`）
- [x] `src/types/search.ts`：`SearchResult` + `ResultKind` TypeScript 型別
- [x] `CommandPalette.tsx`：改用 `search.query` 統一搜尋，結果顯示 App / File badge
- [x] `lib.rs`：啟動時背景 pre-scan apps（消除首次搜尋冷啟動延遲）

**待補齊**
- [ ] Tantivy 離線索引（Everything 不可用時的完整檔案搜尋後端）
- [ ] `search.backend` 前端顯示（在搜尋框角落顯示目前使用的後端）
- [ ] `cmd_rebuild_search_index`：手動重建 Tantivy 索引
- [ ] `default_config.toml` 加入 `[search]` 區塊（可設定索引路徑、排除目錄）
- [ ] 單元測試：`test_backend_selection_*`

### 其他 Phase 2 功能
- [ ] 計算機（Ctrl+=）：單位換算、進位換算
- [ ] 剪貼簿歷史（Ctrl+Shift+V）：圖片預覽、歷史管理
- [ ] 系統控制（Win+S）：音量、亮度、WiFi

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

## 持續維護規則

> 以下為跨 Phase 的長期維護任務，每次對應事件發生時必須執行。

- [ ] **快捷鍵文件同步**：每次新增、修改或移除任何快捷鍵後，必須同步更新 `README.md` 的「⌨️ 快捷鍵速查」區塊（已實作 / 規劃中分組），確保文件與程式碼一致

---

## 阻塞中

- 無
