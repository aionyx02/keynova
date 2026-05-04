# tasks.md — 任務追蹤

> 完成任務請打 `[x]`，並在 memory.md 更新「上次完成」與「下一步」。
> 阻塞中的任務請移至「阻塞中」區塊並說明原因。

---

## ✅ 已完成（壓縮摘要）

| Phase / Bug | 完成內容 |
|-------------|---------|
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
| BUG-3 | 設定儲存過頻 + 型別字串化：onBlur 儲存、toml_value_str() 偵測型別，數字欄位正確寫出 |
| BUG-5 | 未使用模組清理 + 包體積：TerminalView/useTerminal/App.css 刪除，xterm.js 改 React.lazy |
| BUG-6 | 漸進式 ESC：有 panel → 清除面板；query 非空 → 清空；query 空 → 隱藏視窗 |
| BUG-7 | `/command` Tab 補全：選中指令 → Tab → 填入 query，不觸發 focus 跳出 |
| BUG-8 | 設定實際生效：terminal init 讀 font_size/scrollback；W/A/S/D 每次按鍵讀 step_size |
| BUG-9 | 快捷鍵欄位捕捉：readOnly + onKeyDown 格式化組合鍵（Ctrl+K 等），立即儲存 |
| BUG-11 | 終端執行 `ipconfig` 後 ESC 無反應：實測確認實體 ESC 會先進 xterm custom key handler 的 `keyup`，不是 DOM/window `keydown`；改為 Escape `keydown` / `keyup` 皆退出，並保留 terminal data `\x1b` / `\x1b[O` 保底、吞掉 focus-in `\x1b[I`；移除 output RAF 推測修補；`npm run build` / `npm run lint` / `cargo check --manifest-path src-tauri/Cargo.toml` 通過 |

---

## 待手動驗收

> 程式碼已修正，仍需手動測試確認。

| Bug | 待驗收項目 |
|-----|-----------|
| BUG-1 | DevTools → Performance，確認搜尋無多餘 IPC round-trip |
| BUG-2 | 呼叫 `hotkey.register` IPC 收到明確的 `NotImplemented` 錯誤 |
| BUG-4 | DevTools Console 無 CSP violation；所有功能在 CSP 啟用後仍正常 |
| BUG-11 | 進入 `>` 終端後執行 `ipconfig`，實體 ESC 應立即退回搜尋模式且視窗不隱藏 |

---

## 進行中

### BUG-10 /setting 儲存可靠性 + 生效時機

**受影響檔案**：`SettingPanel.tsx`、`config_manager.rs`

- [x] `SettingPanel.tsx`：`saveValue` 成功後顯示 "✓ 已儲存" 閃爍提示（1.5s 後消失）
- [x] `SettingPanel.tsx`：每個 section 加入「生效時機」小標籤
- [x] `SettingPanel.tsx`：saveError 改為不自動清空（目前每次 save 前清空）
- [x] `config_manager.rs`：`persist()` 失敗時 eprintln 詳細路徑與錯誤，方便 DevTools 追蹤

**驗收**
- [x] 在欄位輸入值後離焦，出現 "✓ 已儲存" 提示
- [x] 每個欄位旁有明確「何時生效」標籤
- [x] `persist()` 失敗時，前端 saveError 顯示具體錯誤訊息

---

### FEAT-1 `/setting [key] [value]` Minecraft 風格命令

**受影響檔案**：`CommandPalette.tsx`、`builtin_cmd.rs`、`lib.rs`

命令規格：
- `/setting` → 開啟設定面板（不變）
- `/setting <key>` → 顯示目前值（Inline）
- `/setting <key> <value>` → 設定並確認（Inline `✓ key = value`）

- [x] `CommandPalette.tsx`：rawInput 拆分 cmdName + cmdArgs；filtering 只比對 cmdName；Enter/onSelect 傳 cmdArgs
- [x] `builtin_cmd.rs`：`BuiltinCmdHandler` 注入 `ConfigManager`；`setting + args` 特殊處理
- [x] `lib.rs`：`BuiltinCmdHandler::new` 傳入 `Arc::clone(&config_manager)`

**驗收**
- [x] 輸入 `/setting terminal.font_size 18` 按 Enter → 回傳 `✓ terminal.font_size = 18`，config.toml 更新
- [x] 輸入 `/setting terminal.font_size` 按 Enter → 回傳目前值
- [x] 輸入 `/setting` 按 Enter → 仍開啟面板

---

### FEAT-2 `keynova start/down/reload` + 設定熱重載（免重啟）

**架構設計**

- 控制平面：新增 `keynova` CLI binary + App 內 local socket 控制伺服器
- 命令協議：`start / down / reload / status`，JSON request/response
- 單實例策略：`start` 先探測端點；已執行則 focus，未執行才啟動 GUI
- reload 語意：只重載 config + 套用到 runtime，不整個 restart
- 自動重載：`setting.set` 後與外部 config.toml 變更，統一走同一條 reload pipeline
- 可觀測性：reload 後送出 `config.reloaded` / `config.reload_failed` 事件給前端

**受影響檔案（預計）**：`src-tauri/src/main.rs`、`lib.rs`、`core/config_manager.rs`、`handlers/setting.rs`、`SettingPanel.tsx`、`src/hooks/useEvents.ts`、`Cargo.toml`

- [x] A1. CLI 入口：`src-tauri/src/bin/keynova.rs`（建議 `clap`），`start/down/reload/status` 子命令
- [x] A2. 控制通道模組：`core/control_plane.rs`，local socket server/client + request/response schema
- [x] A3. App 啟動時啟動 control server（背景 thread）；`down` graceful exit、`reload` 觸發 runtime reload
- [x] A4. `ConfigManager` 擴充 `reload_from_disk()`、`snapshot()`、`diff()`
- [x] A5. 新增 `ConfigApplier`：hotkeys 重新綁定、mouse/terminal/launcher 即時更新策略
- [x] A6. 檔案監看（`notify` crate）監聽 `%APPDATA%\Keynova\config.toml`，外部修改自動 reload
- [x] A7. `setting.set` 完成後觸發同一條 apply pipeline（不只寫檔）
- [x] A8. 事件橋接：前端 `SettingPanel` 顯示「已套用 / 套用失敗」
- [x] A9. 命令層：新增 `/reload`、`/down` 供 App 內命令列操作
- [x] A10. 文件：`README.md` 補 `keynova` CLI 用法、reload 生效矩陣、失敗排查

**驗收**
- [x] `keynova start`：未啟動時啟動；已啟動時 bring-to-front，不重複開實例
- [x] `keynova down`：3 秒內關閉常駐進程與 tray
- [x] `keynova reload`：修改 `hotkeys.app_launcher` 後無需重啟即生效
- [x] 直接編輯 config.toml 存檔，App 於 1 秒內自動套用並顯示提示
- [x] reload 失敗時前端有錯誤訊息、stderr 有路徑與原因，不破壞現有配置

---

---

### FEAT-3 指令參數提示（Command Argument Hints）

**受影響檔案**：`builtin_command_registry.rs`、`builtin_cmd.rs`、`useCommands.ts`、`CommandPalette.tsx`、`CommandSuggestions.tsx`

- [x] `BuiltinCommand` trait 加 `args_hint()` 預設方法；`CommandMeta` 加 `args_hint` 欄位
- [x] `SettingCommand` 實作 `args_hint() -> Some("[key] [value]")`
- [x] `BuiltinCmdHandler` 新增 `cmd.suggest_args { name, partial }` handler（setting 回傳 config keys）
- [x] `useCommands.ts`：`CommandMeta` 加 `args_hint?: string`；新增 `suggestArgs()` function
- [x] `CommandSuggestions.tsx`：指令旁顯示 `args_hint` 標籤
- [x] `CommandPalette.tsx`：偵測 args phase（spaceIdx !== -1 且 cmdName 完全匹配）；hint bar + arg suggestions 下拉；↑↓ 選擇、Tab 填入、Enter 執行

**驗收**
- [x] 輸入 `/setting`（出現在指令列表時）旁顯示 `[key] [value]` 標籤
- [x] 輸入 `/setting ` 出現 hint bar 和 config keys 補全列表
- [x] 輸入 `/setting terminal.` 過濾只顯示 terminal.* keys
- [x] Tab 填入選中的 key（後面自動加空格）；Enter 執行指令

---

### FEAT-4 終端機退出後內容保留（Terminal Persistence）

**受影響檔案**：`CommandPalette.tsx`、`TerminalPanel.tsx`

- [x] `CommandPalette.tsx`：`terminalMounted` state；第一次進入 `>` 時在 `handleQueryChange` 設為 true
- [x] `CommandPalette.tsx`：改用 CSS `display: none/block` 控制終端可見性，不再 unmount
- [x] `TerminalPanel.tsx`：新增 `isActive: boolean` prop；`isActive` 變 true 時重新 fit + focus

**驗收**
- [x] 進入終端輸入指令後，按 ESC 退回搜尋模式，再按 `> ` 重新進入，歷史記錄保留
- [x] PTY session 在背景繼續存活（不關閉）

---

### FEAT-5 設定表格鍵盤導航（Settings Keyboard Navigation）

**受影響檔案**：`SettingPanel.tsx`

- [x] `inputRefs` array ref 儲存所有欄位 DOM refs
- [x] `useEffect([entries.length, activeSection])`：entries 載入或 section 切換時自動聚焦第一欄
- [x] `switchSection(dir)` helper：左右切換 tab
- [x] `handleInputKeyDown` 統一 handler：↑↓ 移行、←→ 切 tab（邊界偵測）、Enter 立即儲存、hotkey 欄位方向鍵路由正確

**驗收**
- [x] 開啟 /setting 後第一個欄位自動聚焦
- [x] ↑↓ 在設定欄位間移動焦點
- [x] → 在最後位置觸發切換到下一個 tab；← 同理
- [x] Enter 立即儲存並閃爍 ✓
- [x] hotkey 欄位方向鍵不被捕捉為快捷鍵

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

- [ ] 新增 `ai.*`、`translation.*`、`workspace.*`、`note.*`、`calculator.*`、`history.*`、`system.*` 指令 namespace
- [ ] 擴充 `default_config.toml`：`[features]`、`[ai]`、`[translation]`、`[notes]`、`[history]`、`[system]`
- [ ] `CommandSuggestions` 新增 `/ai`、`/tr`、`/note`、`/cal`、`/history`、`/system` discoverability
- [ ] 建立 Phase 3 測試骨架：Rust 單元測試、前端 hook/component 測試、端到端驗收腳本
- [ ] 新增 ADR：AI provider abstraction、note 同步策略、system-control 權限範圍

### Phase 3.1 翻譯（/tr）

- [ ] 命令規格：`/tr <src> <dst> <text>` 與 `/tr default <text>`
- [ ] `TranslationManager` + `TranslationHandler`，provider trait（先接 Claude，保留可替換）
- [ ] 前端 i18n：抽離 UI 文案、`zh-TW` / `en-US` 資源、語言切換設定持久化
- [ ] 翻譯結果 UX：顯示來源語言偵測、目標語言、一鍵複製

### Phase 3.2 AI 助理（/ai）

- [ ] `AiManager` + `AiHandler` + `AiProvider` 抽象（Claude 為預設）
- [ ] 進入 `/ai` 後 `>` 切換為 AI 對話模式（多輪、取消、錯誤提示）
- [ ] MVP：程式碼解釋、文件草稿、快速提問
- [ ] 安全限制：timeout/retry/token 上限、API key 設定檢查

### Phase 3.3 虛擬工作區（Ctrl+Alt+1/2/3）

- [ ] Workspace domain model（3 槽位）與持久化
- [ ] Hotkey 註冊/切換 + 衝突檢查
- [ ] 狀態隔離：query、search results、命令歷史按 workspace 分離
- [ ] UI 指示：目前 workspace 編號與快速切換提示

### Phase 3.4 快速筆記（/note）

- [ ] `NoteManager` + `NoteHandler`：Markdown 讀寫、命名規則、衝突處理
- [ ] 內建 nano 風格編輯器 MVP（快捷鍵、儲存提示、離開確認）
- [ ] 外部編輯器同步：檔案監看與重新載入

### Phase 3.5 計算機（/cal）

- [ ] `mathjs` + 安全 sandbox；支援 `+-*/`、括號、LaTeX 基本公式
- [ ] 進階：單位換算、進位轉換（bin/dec/hex）、歷史重算
- [ ] UI：輸入即時計算、結果可複製、錯誤可讀化

### Phase 3.6 剪貼簿歷史（/history）

- [ ] Clipboard watcher（Windows 先行）+ `HistoryManager`（容量上限、去重）
- [ ] 支援文字與圖片 metadata
- [ ] `/history` 面板：搜尋、貼回、釘選、刪除、隱私設定

### Phase 3.7 系統控制（/system）

- [ ] 命令規格：音量、亮度、WiFi 查詢/調整/切換
- [ ] `SystemManager` + `SystemHandler`（Windows API；其他平台回傳 `NotImplemented`）

### Phase 3.8 整合、效能、發版準備（Week 18）

- [ ] `npm run lint`、`cargo clippy -- -D warnings`、`cargo test` 全綠
- [ ] `npm run tauri build` 包體積檢查（目標 ~50MB）
- [ ] Regression：Search / Terminal / Command / Setting / Hotkey / Mouse 全面回歸
- [ ] 更新文件：`README.md`、`memory.md`

---

## Phase 4 — v3.0+
- [ ] 個人化（主題、字體、配色方案）
- [ ] 工作區同步（OneDrive/Google Drive）
- [ ] Plugin System（JS/Python 擴展）
- [ ] 流程自動化（類似 AutoHotkey 的腳本功能）
- [ ] 跨平台優化（Linux/macOS 支援、WSL 深度整合）
- [ ] 公開 API

---

## 持續維護規則

- [ ] **快捷鍵文件同步**：每次新增、修改或移除任何快捷鍵後，同步更新 `README.md` 的「⌨️ 快捷鍵速查」區塊

---

## 阻塞中

- 無
