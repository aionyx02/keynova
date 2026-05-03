# tasks.md — 任務追蹤

> 完成任務請打 `[x]`，並在 memory.md 更新「上次完成」與「下一步」。
> 阻塞中的任務請移至「阻塞中」區塊並說明原因。

---

## ✅ 已完成（壓縮摘要）

| Phase | 完成內容 |
|-------|---------|
| Phase 0 | 專案初始化：Tauri scaffold、ESLint/Prettier/Tailwind、CommandRouter/EventBus、IPC 基本通訊、所有規格文件 |
| Phase 1.0 | 前置依賴（zustand、xterm、portable-pty、windows crate）、目錄結構、default_config.toml |
| Phase 1.1 | App Launcher：AppManager、LauncherHandler、CommandPalette 搜尋 + 鍵盤導航，Ctrl+K 全域熱鍵 |
| Phase 1.2 | 全域熱鍵系統：HotkeyManager、HotkeyHandler、tauri-plugin-global-shortcut 整合 |
| Phase 1.3 | PTY 終端：TerminalManager、TerminalHandler、TerminalView（xterm.js）、EventBus terminal.output 橋接 |
| Phase 1.4 | 鍵盤滑鼠控制：MouseManager、MouseHandler、SendInput WinAPI、Ctrl+Alt+M/W/A/S/D/Enter 全域熱鍵 |
| Phase 1.5 | Flow Launcher 風格 UI：無邊框透明視窗、系統托盤、Ctrl+K 常駐、CommandPalette 重設計 |
| Phase 1.6 | 搜尋系統：SearchManager（Everything IPC + AppCache fallback）、SearchHandler、SearchResult 模型 |
| Phase 2.A | 三模式輸入（Search / `>` Terminal / `/` Command）、xterm.js PTY pre-warm、TerminalPanel、useInputMode |
| Phase 2.B | BuiltinCommandRegistry、HelpCommand、SettingCommand、BuiltinCmdHandler、SettingHandler、ConfigManager TOML I/O、CommandSuggestions、SettingPanel（4 tab）、PanelRegistry |
| Task 1 | 全域檔案搜尋擴展：D–Z 槽 + WSL home（wsl.exe --list + UNC）、SKIP_DIRS 補充 |

---

## 已知問題與技術債

> 以下問題由使用者審查後確認，優先於新功能開發。

### BUG-1 搜尋路徑卡 UI + 競態條件

**受影響檔案**：`CommandPalette.tsx`、`search_manager.rs`、`windows.rs`

- [x] `CommandPalette.tsx`：搜尋輸入加 200ms debounce + requestId 競態防護
- [x] `windows.rs`：背景索引任務（build_file_index）+ 記憶體快取（FILE_CACHE）
- [x] `search_manager.rs`：fallback 只查快取，不做即時遞迴掃碟
- [x] `lib.rs`：setup 啟動背景索引 thread

**驗收**
- [x] 快速連續輸入 5 個字元，UI 不卡頓，結果只更新一次
- [ ] DevTools → Performance，確認無多餘的 IPC round-trip

---

### BUG-2 Hotkey 設定與實作脫節

- [x] `lib.rs`：setup_global_shortcuts 讀取 config_manager，動態注冊（含 fallback 回預設值機制）
- [x] `windows.rs`：WinAPI RegisterHotKey 骨架直接移除
- [x] `handlers/hotkey.rs`：hotkey.register / unregister IPC 回傳明確的 NotImplemented
- [x] 確認只保留 tauri-plugin-global-shortcut 一套 hotkey 系統

**驗收**
- [ ] 在 `/setting` 修改 `hotkeys.app_launcher`，重啟後新快捷鍵生效
- [ ] 呼叫 `hotkey.register` IPC 收到明確的 `NotImplemented` 錯誤

---

### BUG-3 設定儲存過頻 + 型別全被字串化

- [x] `SettingPanel.tsx`：onBlur 才呼叫 setting.set IPC，加入「儲存中…」視覺回饋
- [x] `config_manager.rs`：persist() 重建 [section] 結構，toml_value_str() 偵測型別

**驗收**
- [ ] 連續修改欄位值，只在 blur 時寫入一次
- [ ] `%APPDATA%\Keynova\config.toml` 中數字欄位為 `font_size = 14`，非 `font_size = "14"`

---

### BUG-4 CSP 安全性：被關閉

- [x] `tauri.conf.json`：設定最小 CSP
- [x] `capabilities/default.json`：審查現有權限（已僅保留 window show/hide/focus/size）
- [ ] 確認 xterm.js 在 CSP 限制下正常運作（需手動測試）

**驗收**
- [ ] DevTools Console 無 CSP violation 錯誤
- [ ] 所有功能在 CSP 啟用後仍正常運作

---

### BUG-5 未使用模組 + 首屏包體積 >500KB

- [x] `TerminalView.tsx`、`useTerminal.ts`、`App.css` 確認未引用，已刪除
- [x] `TerminalPanel.tsx`：改為 React.lazy + Suspense，xterm.js 只在進入 `>` 時才載入
- [ ] `npm run build` 確認無 `>500KB chunk` 警告（需手動驗收）

---

### BUG-6 /setting 面板按 ESC 無法回到搜尋框

**受影響檔案**：`CommandPalette.tsx`

- [x] ESC 改為漸進式：有 cmdResult → 清除面板回搜尋模式（不隱藏視窗）；query 非空 → 清空 query；query 空 → 隱藏視窗
- [x] 確保 ESC 在 terminal 模式不受此邏輯影響

**驗收**
- [ ] 進入 `/setting` 後按 ESC 回到空白搜尋框（視窗不關閉）
- [ ] 再按一次 ESC 才關閉視窗

---

### BUG-7 /command 模式下 Tab 無自動補全

**受影響檔案**：`CommandPalette.tsx`

- [x] 在 command 模式 onKeyDown 新增 Tab 鍵處理：將選中指令名稱填入 query（`"/" + cmd.name`）
- [x] Tab 不應觸發 focus 切換（preventDefault）

**驗收**
- [ ] 輸入 `/he` → 選中 `/help` → 按 Tab → query 變為 `/help`

---

### BUG-8 修改 Setting 內容對環境無實際影響

**受影響檔案**：`TerminalPanel.tsx`、`lib.rs`

- [x] `TerminalPanel.tsx`：init() 開始時透過 setting.list_all IPC 讀取 terminal.font_size、terminal.scrollback_lines，再以讀取值建立 Terminal
- [x] `lib.rs`：W/A/S/D 快捷鍵 lambda 改為每次觸發時從 _config_manager 讀取 mouse_control.step_size（取代 const STEP 硬編碼）

**驗收**
- [ ] 在 `/setting` 修改 `terminal.font_size = 18`，重新進入 `>` 終端，字體大小改變
- [ ] 在 `/setting` 修改 `mouse_control.step_size = 30`，游標移動距離翻倍

---

### BUG-9 Setting 快捷鍵欄位需手動輸入按鍵名稱

**受影響檔案**：`SettingPanel.tsx`

- [x] `hotkeys.*` 欄位設為 readOnly，onKeyDown 捕捉組合鍵，自動填入格式化字串（如 `Ctrl+K`）
- [x] 純修飾鍵按下不觸發填入，等待主鍵
- [x] 填入後立即透過 saveValue() 寫入，不依賴 onBlur

**驗收**
- [ ] 點選 `hotkeys.app_launcher` 欄位後按 `Ctrl+J`，欄位自動顯示 `Ctrl+J`
- [ ] 離開欄位後儲存到 config.toml

---

## Phase 2 — 待補齊

- [ ] Tantivy 離線索引（Everything 不可用時的完整檔案搜尋後端）
- [ ] `search.backend` 前端顯示（搜尋框角落顯示目前使用的後端）
- [ ] `cmd_rebuild_search_index`：手動重建 Tantivy 索引
- [ ] `default_config.toml` 加入 `[search]` 區塊（可設定索引路徑、排除目錄）
- [ ] 單元測試：`test_backend_selection_*`

---

## Phase 3 — v2.0（weeks 13–18）

### Phase 3.0 基線與共用能力（Week 13）

- [ ] 新增 `ai.*`、`translation.*`、`workspace.*`、`note.*`、`calculator.*`、`history.*`、`system.*` 指令 namespace（Handler + Manager + typed IPC）
- [ ] 擴充 `default_config.toml`：`[features]`、`[ai]`、`[translation]`、`[notes]`、`[history]`、`[system]`
- [ ] `CommandSuggestions` 新增 `/ai`、`/tr`、`/note`、`/cal`、`/history`、`/system` 的 discoverability
- [ ] 建立 Phase 3 測試骨架：Rust 單元測試、前端 hook/component 測試、端到端驗收腳本
- [ ] 新增 ADR：AI provider abstraction、note 同步策略、system-control 權限範圍

### Phase 3.1 翻譯與國際化（/tr）

- [ ] 命令規格：`/tr <source_language> <destination> <text>` 與 `/tr default <text>`
- [ ] 新增 `TranslationManager` + `TranslationHandler`，定義 provider trait（先接 Claude，保留可替換）
- [ ] 前端 i18n：抽離 UI 文案、建立 `zh-TW` / `en-US` 資源、語言切換設定持久化
- [ ] 翻譯結果 UX：顯示來源語言偵測、目標語言、可一鍵複製
- [ ] 驗收：命令解析測試、翻譯成功/失敗路徑測試、UI 語言切換測試

### Phase 3.2 AI 助理（/ai）

- [ ] 新增 `AiManager` + `AiHandler` + `AiProvider` 抽象（Claude 為預設 provider）
- [ ] 進入 `/ai` 後，`>` 切換為 AI 對話模式（多輪對話、取消請求、錯誤提示）
- [ ] MVP 能力：程式碼解釋、文件草稿生成、快速提問
- [ ] 安全限制：timeout/retry/token 上限、API key 設定檢查
- [ ] 驗收：mock provider 測試 + 斷網/逾時/速率限制實機測試

### Phase 3.3 虛擬工作區（Ctrl+Alt+1/2/3）

- [ ] 建立 Workspace domain model（3 槽位）與持久化
- [ ] Hotkey 註冊/切換：`Ctrl+Alt+1/2/3`，含衝突檢查與錯誤提示
- [ ] 狀態隔離：query、search results、命令歷史按 workspace 分離
- [ ] UI 指示：顯示目前 workspace 編號與快速切換提示
- [ ] 驗收：快速切換不丟狀態、重啟後狀態可恢復

### Phase 3.4 快速筆記（/note）

- [ ] 命令規格：`/note <name>` 建立/開啟、`/note` 顯示列表並支援續編與刪除
- [ ] 新增 `NoteManager` + `NoteHandler`：Markdown 讀寫、命名規則、衝突處理
- [ ] 內建 nano 風格編輯器 MVP（快捷鍵、儲存提示、離開確認）
- [ ] 外部編輯器同步：先做檔案監看與重新載入
- [ ] 驗收：新建、續編、刪除、外部修改回寫流程驗證

### Phase 3.5 計算機（/cal）

- [ ] 命令規格：`/cal <expression>`，支援 `+-*/`、括號、LaTeX 基本公式
- [ ] 計算引擎整合：`mathjs` + 安全 sandbox（禁止任意程式執行）
- [ ] 進階能力：單位換算、進位轉換（bin/dec/hex）、歷史重算
- [ ] UI：輸入即時計算、結果可複製、錯誤訊息可讀化
- [ ] 驗收：公式、單位、進位三類測試案例覆蓋

### Phase 3.6 剪貼簿歷史（/history）

- [ ] Clipboard watcher（Windows 先行）與 `HistoryManager` 儲存策略（容量上限、去重）
- [ ] 支援文字與圖片 metadata（縮圖、大小、時間）
- [ ] `/history` 面板：搜尋、貼回、釘選、刪除
- [ ] 隱私設定：排除敏感應用、清空歷史、開關功能
- [ ] 驗收：大量複製壓測、重啟後資料一致性、圖片預覽可用

### Phase 3.7 系統控制（/system）

- [ ] 命令規格：音量、亮度、WiFi 的查詢/調整/切換
- [ ] 新增 `SystemManager` + `SystemHandler`（Windows API；Linux/macOS 先回傳 `NotImplemented`）
- [ ] 權限與錯誤回報：無權限、裝置不可用、平台不支援
- [ ] UI 回饋：操作成功值、失敗原因、恢復建議
- [ ] 驗收：每個命令至少一組成功與一組失敗路徑測試

### Phase 3.8 整合、效能、發版準備（Week 18）

- [ ] `npm run lint`、`cargo clippy -- -D warnings`、`cargo test` 全綠
- [ ] `npm run tauri build` 包體積檢查（目標維持約 50MB）
- [ ] Regression：Search / Terminal / Command / Setting / Hotkey / Mouse 全面回歸
- [ ] 更新文件：`README.md`、`memory.md`
- [ ] 發版清單：feature flag 預設、風險開關、回滾方案

---

## Phase 4 — v3.0+

- [ ] Plugin System（JS/Python 擴展）
- [ ] 雲端同步
- [ ] 公開 API

---

## 持續維護規則

- [ ] **快捷鍵文件同步**：每次新增、修改或移除任何快捷鍵後，同步更新 `README.md` 的「⌨️ 快捷鍵速查」區塊

---

## 阻塞中

- 無