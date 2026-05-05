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
| BUG-10 | /setting 儲存可靠性：onBlur 儲存 + "✓ 已儲存" 提示、生效時機標籤、persist() 失敗詳細 stderr |
| BUG-11 | 終端 ESC 修復：keydown/keyup 皆退出，保留 `\x1b`/`\x1b[O` 保底，吞掉 focus-in `\x1b[I` |
| FEAT-1 | `/setting [key] [value]` Minecraft 風格命令：rawInput 拆分、BuiltinCmdHandler 注入 ConfigManager |
| FEAT-2 | `keynova start/down/reload` CLI + control plane loopback socket + config 熱重載 + notify 檔案監看 |
| FEAT-3 | 指令參數提示：args_hint trait、hint bar + arg suggestions 下拉、↑↓ Tab Enter |
| FEAT-4 | 終端機退出後內容保留：CSS display 控制可見性（不 unmount），isActive re-fit/focus |
| FEAT-5 | 設定表格鍵盤導航：inputRefs、auto-focus、↑↓ 移行、←→ 切 tab、Enter 立即儲存 |
| Phase 3.0 | ai/translation/workspace/note/calculator/history/system namespace + config 區塊 + ADR-011~013 |
| Phase 3.1 | TranslationManager + TranslationHandler（reqwest+EventBus）+ TranslationPanel + i18n zh-TW/en-US |
| Phase 3.2 | AiManager + AiHandler（Claude Sonnet 4.6）+ AiPanel 多輪對話 |
| Phase 3.3 | WorkspaceManager 3槽位 + Ctrl+Alt+1/2/3 + WorkspaceIndicator |
| Phase 3.4 | NoteManager + NoteHandler + NoteEditor（debounce auto-save）；外部 file watch defer Phase 4 |
| Phase 3.5 | 純 Rust recursive-descent 計算機：+-*/%^、unit/base 換算 + CalculatorPanel |
| Phase 3.6 | HistoryManager（FNV-1a dedup）+ PowerShell clipboard watcher + HistoryPanel |
| Phase 3.7 | SystemManager（COM volume + PS WMI brightness + netsh wifi）+ SystemPanel |
| Phase 3.8 | lint/tsc/clippy/test/vite build 全綠；npm run tauri build + regression 待手動執行 |

---

## 待手動驗收

| Bug | 待驗收項目 |
|-----|-----------|
| BUG-1 | DevTools → Performance，確認搜尋無多餘 IPC round-trip |
| BUG-2 | 呼叫 `hotkey.register` IPC 收到明確的 `NotImplemented` 錯誤 |
| BUG-4 | DevTools Console 無 CSP violation；所有功能在 CSP 啟用後仍正常 |
| BUG-11 | 進入 `>` 終端後執行 `ipconfig`，實體 ESC 應立即退回搜尋模式且視窗不隱藏 |
| Phase 3.8 | `npm run tauri build` 包體積確認（目標 ~50MB）；Search / Terminal / Command / Setting / Hotkey / Mouse 全面手動回歸 |

---

## 進行中

### ~~BUG-12~~ ✅ ESC 在 panel 模式（/ai、/command 等）無反應
- [x] `PanelProps.onClose` 加入所有 panel；各 panel textarea/input 攔截 Escape 呼叫 `onClose()`
- [x] CommandPalette 傳 `handlePanelClose` 並加 Suspense 包裝
- [x] ESC handler 改為 `deps=[setQuery]` 只註冊一次；refs 透過 `useLayoutEffect`（無 deps）同步，解決 stale closure
- [ ] 驗收：/ai、/tr、/note、/cal、/history、/system 面板按 ESC 均可退出，回到搜尋模式

---

### ~~BUG-13~~ ✅ /command 模式，↑ 鍵應返回頂部輸入框
- [x] ArrowUp 邏輯改為 `(i <= 0 ? -1 : i - 1)`，消除 0↔-1 振盪
- [ ] 驗收：/command 最頂項按 ↑，焦點回到搜尋框

---

### ~~BUG-14~~ ✅ 搜尋模式，↓ 鍵選擇結果時搜尋框不跟隨滾動
- [x] 還原內層捲動（`max-h` 放在 `<ul>`），搜尋框固定在上方，不使用 sticky
- [x] selected `<li>` 加 `scrollIntoView({ block: "nearest" })`
- [ ] 驗收：搜尋結果超出視窗高度時，↓ 鍵選到下方項目，輸入框仍可見

---

### FEAT-6 `/model_download` + `/model_list` + /ai 多 Provider 支援

**目標**：新增兩個獨立指令，讓使用者無需手動設定就能快速上手本地 AI 或 API。

> **跨功能原則**：`/ai`、`/tr`（本地模型模式）等所有需要 AI 的功能，都統一讀取 `model_manager` 的 **active model**（由 `/model_download` 或 `/model_list` 設定），不各自硬寫 provider。

---

#### 指令規格

**`/model_download`**
1. 開啟 `ModelDownloadPanel`，**偵測當前硬體**（RAM、GPU VRAM），顯示：
   - 推薦模型清單（依硬體篩選，列出名稱、大小、供應商、適配評級）
   - 分隔線：「或使用 API」→ 列出 Claude / OpenAI-compatible 選項（需填 key）
2. ↑↓ 選擇項目，Enter 確認
3. 選到 **本地模型**：先呼叫 `model.check { name }` 確認 Ollama 是否已有此模型
   - 已有 → 直接設為使用中（`ai.provider = ollama, ai.model = name`），顯示 "✓ 已就緒"
   - 沒有 → 確認下載，顯示即時進度條（`model-pull-progress` 事件），完成後自動啟用
4. 選到 **API provider** → 提示輸入 key，驗證後儲存至 config，切換 provider

**`/model_list`**
- 開啟 `ModelListPanel`，顯示所有可用模型（已下載 + 已設定 API）
- 欄位：名稱、版本、供應商、大小/類型、狀態（使用中 / 已下載 / API）
- ↑↓ 導航，Enter 切換為使用中，Delete 刪除本地模型

---

#### 硬體偵測與推薦邏輯

```
RAM < 8 GB   → 推薦：qwen2.5:1.5b（1.0 GB）, llama3.2:1b（1.3 GB）
RAM 8–16 GB  → 推薦：llama3.2:3b（2.0 GB）, qwen2.5:3b（2.0 GB）, gemma3:4b（3.3 GB）
RAM ≥ 16 GB 或 GPU VRAM ≥ 6 GB → 推薦：llama3.1:8b（4.7 GB）, qwen2.5:7b（4.7 GB）, mistral:7b（4.1 GB）
```

GPU VRAM 偵測（Windows）：`wmic path Win32_VideoController get AdapterRAM` 或 DXGI

---

#### 架構設計

- `AiProvider` enum：`Claude { api_key, model }` / `Ollama { base_url, model }` / `OpenAI { api_key, base_url, model }`
- `managers/model_manager.rs`（新）：`detect_hardware() -> HardwareInfo`、`recommend_models(hw) -> Vec<ModelCandidate>`、`list_local()` / `pull(name)` / `delete(name)` / `check(name)`
- `handlers/model.rs`（新）：namespace `model`，commands: `detect_hardware`、`recommend`、`list_local`、`pull { name }`、`delete { name }`、`check { name }`、`set_active { provider, model }`
- EventBus：`model-pull-progress { name, completed_bytes, total_bytes, percent }`、`model-pull-done { name }`、`model-pull-error { name, error }`
- `ModelCommand`（`model_download`）、`ModelListCommand`（`model_list`）加入 `BuiltinCommandRegistry`
- `ModelDownloadPanel.tsx`、`ModelListPanel.tsx` 加入 `PanelRegistry`

**受影響檔案**：`managers/model_manager.rs`（新）、`handlers/model.rs`（新）、`handlers/mod.rs`、`managers/mod.rs`、`handlers/builtin_cmd.rs`、`lib.rs`、`handlers/ai.rs`、`managers/ai_manager.rs`、`default_config.toml`、`src/components/ModelDownloadPanel.tsx`（新）、`src/components/ModelListPanel.tsx`（新）、`src/components/panel/PanelRegistry.tsx`

#### 子任務

**後端**
- [ ] `managers/model_manager.rs`：`HardwareInfo { ram_mb, vram_mb }`、`detect_hardware()`（Windows: wmic）、`ModelCandidate { name, size_gb, provider, rating }`、`recommend_models()`
- [ ] `managers/model_manager.rs`：`list_local()` 呼叫 `GET /api/tags`、`check(name)` 確認模型存在、`pull(name, tx: EventSender)` 串流下載進度、`delete(name)`
- [ ] `handlers/model.rs`：`detect_hardware`、`recommend`、`list_local`、`check { name }`、`pull { name }`（背景 thread + EventBus）、`delete { name }`、`set_active { provider, model }`
- [ ] `managers/ai_manager.rs`：AiProvider enum 三種 variant，`build_provider()` 工廠函式從 config 讀取
- [ ] `default_config.toml`：`[ai] provider = "claude"` / `ollama_url = "http://localhost:11434"`

**前端**
- [ ] `ModelDownloadPanel.tsx`：硬體資訊列（RAM/VRAM badge）、推薦模型列表（評級星號）、API 選項區、Enter 確認 → check → 下載進度條
- [ ] `ModelListPanel.tsx`：模型表格（名稱/版本/供應商/大小/狀態）、Enter 切換使用中、Delete 刪除
- [ ] `PanelRegistry.tsx`：加入 `model_download` → `ModelDownloadPanel`、`model_list` → `ModelListPanel`
- [ ] `builtin_cmd.rs`：`ModelDownloadCommand`（name="model_download"）、`ModelListCommand`（name="model_list"）

**驗收**
- [ ] 輸入 `/model_download` → 面板顯示當前 RAM/VRAM，列出適配模型
- [ ] 選擇未下載模型 Enter → 確認下載 → 進度條 → 完成後「✓ 已啟用 llama3.2:3b」
- [ ] 選擇已下載模型 Enter → 直接切換，不重複下載
- [ ] 選擇 Claude API → 提示輸入 key → 儲存 → provider 切換
- [ ] 輸入 `/model_list` → 顯示已下載模型 + 已設定 API，Enter 切換使用中

---

### FEAT-7 /tr 免費優先翻譯（Google 免費端點 + 本地模型 + 付費可選）

**設計原則**：零設定即可使用，任何 provider 都不強制花錢。

#### Provider 優先級（由上到下自動回退）

| 層級 | Provider | 費用 | 條件 |
|------|----------|------|------|
| 1（預設） | Google Translate 免費端點 | 免費，無需 key | 無條件可用（rate limit 約 500 req/hr） |
| 2（自動升級） | 本地 Ollama 模型 | 免費 | `/model_list` 中有 active 本地模型時可選 |
| 3（可選付費） | Google Cloud Translation API v2 | 付費，需 key | 設定 `translation.google_api_key` 後啟用 |
| 4（可選付費） | Claude（現有） | 付費，需 key | 設定 `translation.provider = "claude"` |

> 預設啟動不需要任何設定，使用者可依需求自行升級，**不強迫任何付費**。

#### 免費端點規格

```
GET https://translate.googleapis.com/translate_a/single
  ?client=gtx&sl={src}&tl={dst}&dt=t&q={encoded_text}
```
回應格式：`[[["譯文","原文",...],...], null, "偵測到的語言"]`

#### Ollama 翻譯方案

- 使用 active model（來自 `/model_list`）
- System prompt：`"You are a translator. Translate the following {src} text to {dst}. Output only the translation, no explanation."`
- 適合長文、隱私要求高的場景（完全離線）

#### 受影響檔案

`managers/translation_manager.rs`、`handlers/translation.rs`、`default_config.toml`、`TranslationPanel.tsx`

#### 子任務

- [ ] `TranslationProvider` enum：`GoogleFree`、`OllamaLocal { model }`、`GoogleCloud { api_key }`、`Claude { api_key, model }`
- [ ] `GoogleFreeProvider`：reqwest GET 免費端點，解析巢狀陣列回應；rate limit 錯誤時回傳明確訊息
- [ ] `OllamaLocalProvider`：透過 `/api/chat` 傳送翻譯 prompt，使用 active model
- [ ] `translation_manager.rs`：`auto_select_provider()` — 無 key 時優先 GoogleFree，有 active local model 時提供 Ollama 選項
- [ ] `default_config.toml`：`[translation] provider = "google_free"`（預設），`google_api_key = ""`
- [ ] `TranslationPanel.tsx`：右上角 provider badge（「Google 免費」/ 模型名稱 / 「Google Cloud」/ 「Claude」）；rate limit 時顯示 inline 警告 + 一鍵切換 Ollama
- [ ] 驗收：零設定 `/tr en zh-TW hello` → 「你好」；有 Ollama active 時可切換本地翻譯；設 `google_api_key` 自動升級到官方 API

---

## Phase 2 — 待補齊

- [ ] Tantivy 離線索引（Everything 不可用時的完整檔案搜尋後端）
- [ ] `search.backend` 前端顯示（搜尋框角落顯示目前使用的後端）
- [ ] `cmd_rebuild_search_index`：手動重建 Tantivy 索引
- [ ] `default_config.toml` 加入 `[search]` 區塊（可設定索引路徑、排除目錄）
- [ ] 單元測試：`test_backend_selection_*`

---

## Phase 4 — v3.0+
- [ ] 個人化（主題、字體、配色方案）
- [ ] 工作區同步（OneDrive/Google Drive）
- [ ] Plugin System（JS/Python 擴展）
- [ ] 流程自動化（類似 AutoHotkey 的腳本功能）
- [ ] 跨平台優化（Linux/macOS 支援、WSL 深度整合）
- [ ] 公開 API
- [ ] 筆記外部編輯器同步（notify crate 檔案監看）
- [ ] 剪貼簿圖片 metadata 支援

---

## 持續維護規則

- [ ] **快捷鍵文件同步**：每次新增、修改或移除任何快捷鍵後，同步更新 `README.md` 的「⌨️ 快捷鍵速查」區塊

---

## 阻塞中

- 無