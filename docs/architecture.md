---
type: architecture_spec
status: active
priority: p1
updated: 2026-05-13
context_policy: retrieve_only
owner: project
---

# Keynova 系統架構

> Retrieval policy: 只在實作、除錯、架構決策相關需求時擷取「最小必要段落」，不要整份注入 prompt。

**版本：** 1.0  
**最後更新：** 2026-05-13  
**相關 ADR：** `docs/adr/`

---

## 1. 系統概覽

Keynova 是以鍵盤為核心的生產力啟動器，採用 **Tauri 2.x + React 18 + Rust** 實作。目標是讓開發者在 90% 的工作流程操作中不需要使用滑鼠。

### 1.1 六層架構

```
┌─────────────────────────────────────────────────────┐
│  Layer 1: Presentation (React + Tauri WebView)      │
│  CommandPalette, TerminalPanel, FloatingWindow…     │
├─────────────────────────────────────────────────────┤
│  Layer 2: IPC Bridge (Tauri invoke + EventBus)      │
│  cmd_dispatch / cmd_ping / cmd_show_launcher…       │
├─────────────────────────────────────────────────────┤
│  Layer 3: Core Framework                            │
│  CommandRouter / EventBus / ConfigManager /         │
│  BuiltinCommandRegistry / ActionArena /             │
│  KnowledgeStoreHandle / AgentRuntime                │
├─────────────────────────────────────────────────────┤
│  Layer 4: Business Logic (Handlers → Managers)      │
│  launcher, hotkey, terminal, mouse, search,         │
│  ai, agent, note, workspace, translation,           │
│  model, system_control, nvim, automation, plugin    │
├─────────────────────────────────────────────────────┤
│  Layer 5: Indexer / Storage                         │
│  tantivy_index, system_indexer, knowledge_store     │
│  (SQLite), config.toml, notes/                      │
├─────────────────────────────────────────────────────┤
│  Layer 6: Platform                                  │
│  Windows / Linux (X11/Wayland) / macOS              │
│  platform::{windows, linux, macos, mod}             │
└─────────────────────────────────────────────────────┘
```

### 1.2 技術棧

| 層次 | 技術 |
|------|------|
| Frontend | React 18, TypeScript, Zustand |
| Desktop bridge | Tauri 2.x |
| Backend runtime | Rust (stable), Tokio async runtime |
| Local AI | Ollama (via HTTP), llama.cpp (計劃中) |
| Search index | Tantivy (Rust 原生全文索引) |
| Persistent store | SQLite (rusqlite), TOML config |
| Editor integration | Neovim (portable, on-demand download) |

---

## 2. 模組邊界

### 2.1 前端 `src/`

```
src/
├── main.tsx               # React 入口，掛載 <App>
├── App.tsx                # 頂層元件，管理 CommandPalette + FloatingWindow
├── components/
│   ├── CommandPalette.tsx # 核心 UI：搜尋框 + 結果列表
│   ├── CommandSuggestions.tsx
│   ├── FloatingWindow.tsx # 浮動視窗容器
│   ├── TerminalPanel.tsx
│   ├── AiPanel.tsx
│   ├── NoteEditor.tsx
│   ├── ModelDownloadPanel.tsx
│   ├── NvimDownloadPanel.tsx
│   ├── HistoryPanel.tsx
│   ├── SettingPanel.tsx
│   ├── SystemPanel.tsx
│   ├── SystemMonitoringPanel.tsx
│   ├── CalculatorPanel.tsx
│   ├── TranslationPanel.tsx
│   ├── WorkspaceIndicator.tsx
│   └── panel/
│       └── PanelRegistry.tsx  # Panel 名稱 → React.lazy 映射
├── hooks/                 # 自訂 React hooks
├── stores/                # Zustand 狀態管理
├── services/              # IPC/backend API clients
└── types/                 # TypeScript 型別定義
```

**前端邊界規則：**
- 所有後端呼叫必須透過 `ipcDispatch(route, payload)` 統一入口
- 元件不直接操作 Tauri API；透過 service 層封裝
- EventBus 事件透過 `listen(topic, handler)` 訂閱（Tauri event system）

### 2.2 後端 `src-tauri/src/`

```
src-tauri/src/
├── main.rs / lib.rs       # Tauri app 入口
├── app/
│   ├── bootstrap.rs       # 初始化流程
│   ├── dispatch.rs        # IPC 命令分派實作
│   ├── state.rs           # AppState（全域狀態組裝）
│   ├── control_server.rs  # 控制伺服器
│   ├── migration.rs       # 資料遷移
│   ├── shortcuts.rs       # 全域快捷鍵設定
│   ├── tray.rs            # 系統托盤
│   ├── watchers.rs        # 檔案 watcher
│   └── window.rs          # 視窗管理
├── core/
│   ├── command_router.rs  # CommandRouter trait + dispatch
│   ├── event_bus.rs       # AppEvent + broadcast channel
│   ├── config_manager.rs  # TOML config 讀取 + diff
│   ├── knowledge_store.rs # SQLite 非同步 actor
│   ├── agent_runtime.rs   # ReAct agent 迴圈
│   ├── action_registry.rs # ActionArena（短生命周期 action ref）
│   ├── builtin_command_registry.rs
│   ├── automation_engine.rs
│   ├── control_plane.rs
│   ├── search_registry.rs
│   ├── observability.rs
│   ├── plugin_runtime.rs
│   └── ipc_error.rs
├── handlers/              # CommandHandler 實作（每個 namespace 一個）
│   ├── agent/             # Agent handler 子模組
│   ├── ai.rs / model.rs / translation.rs
│   ├── launcher.rs / search.rs / history.rs
│   ├── hotkey.rs / mouse.rs
│   ├── terminal.rs / note.rs / workspace.rs
│   ├── system_control.rs / system_monitoring.rs
│   ├── builtin_cmd.rs / calculator.rs / setting.rs
│   ├── nvim.rs / automation.rs / plugin.rs
│   └── mod.rs
├── managers/              # 業務邏輯（純 Rust，無 Tauri 依賴）
│   ├── ai_manager.rs / model_manager.rs
│   ├── app_manager.rs / system_manager.rs / system_indexer.rs
│   ├── history_manager.rs / search_manager.rs / tantivy_index.rs
│   ├── hotkey_manager.rs / mouse_manager.rs
│   ├── terminal_manager.rs / note_manager.rs / workspace_manager.rs
│   ├── calculator_manager.rs / translation_manager.rs
│   ├── portable_nvim_manager.rs / sandbox_manager.rs
│   └── mod.rs
├── models/                # 共用資料結構（serde）
│   ├── action.rs / agent.rs / app.rs / builtin_command.rs
│   ├── hotkey.rs / plugin.rs / search_result.rs
│   ├── settings_schema.rs / terminal.rs / workflow.rs
│   └── mod.rs
├── platform/              # 平台特定程式碼（條件編譯）
│   ├── windows.rs / linux.rs / macos.rs
│   └── mod.rs
└── platform_dirs.rs       # 平台目錄路徑解析
```

---

## 3. 資料流

### 3.1 主要請求路徑（Request Path）

```
User keystroke
    │
    ▼
CommandPalette (React)
    │  ipcDispatch("namespace.command", payload)
    ▼
Tauri invoke → cmd_dispatch
    │  app/dispatch.rs :: cmd_dispatch_impl()
    ▼
CommandRouter::dispatch(route, payload)
    │  core/command_router.rs
    ▼
CommandHandler::execute(command, payload)
    │  handlers/<namespace>.rs
    ▼
Manager::method()
    │  managers/<feature>_manager.rs
    ▼
Result<Value, String>  ←── 回傳給前端
```

### 3.2 事件推送路徑（Event Push）

```
Manager / AgentRuntime / TerminalManager
    │  event_bus.publish(AppEvent { topic, payload })
    ▼
EventBus (tokio broadcast channel, capacity=256)
    │
    ▼
app/watchers.rs :: event_relay_loop()
    │  tauri::Emitter::emit(legacy_topic, payload)
    ▼
Frontend listen("topic-with-dashes", handler)
    │  React component 更新 state
```

**注意：** EventBus topic 使用點號分隔（`terminal.output`），Tauri emit 時轉為連字號（`terminal-output`）透過 `AppEvent::legacy_tauri_topic()`。

### 3.3 Action 生命周期

```
SearchHandler → ActionArena::insert()  → ActionRef { session_id, generation, id }
                                              │
Frontend receives ActionRef                  │
    │  ipcDispatch("action.run", { action_ref })
    ▼
dispatch.rs :: run_action_command("run", ...)
    │  ActionArena::resolve(action_ref)
    ▼
ActionKind 分派：
  LaunchPath  → launcher.launch
  CommandRoute → dispatch_command()
  OpenPanel   → ActionResult::Panel { name }
  Inline      → ActionResult::Inline { text }
  Noop        → ActionResult::Noop { reason }
    │
    ▼
KnowledgeStore::try_log_action()  (非同步 SQLite 寫入)
```

---

## 4. IPC 契約

### 4.1 統一入口

| Tauri command | 簽章 | 用途 |
|--------------|------|------|
| `cmd_dispatch` | `(route: String, payload: Option<Value>) → Result<Value, IpcError>` | 所有業務命令 |
| `cmd_ping` | `(name: Option<String>) → Result<String, IpcError>` | 健康檢查 |
| `cmd_show_launcher` | `() → Result<(), IpcError>` | 顯示視窗 |
| `cmd_hide_launcher` | `() → Result<(), IpcError>` | 隱藏視窗 |
| `cmd_keep_launcher_open` | `() → Result<(), IpcError>` | 防止視窗自動關閉 |

**Route 格式：** `"namespace.command"`，例如 `"launcher.list"`, `"search.query"`, `"ai.chat"`。

### 4.2 已註冊 Handler namespace 一覽

| namespace | handler 檔案 | 主要功能 |
|-----------|-------------|---------|
| `system` | system_control.rs | ping, shutdown, info |
| `launcher` | launcher.rs | list, launch, search apps |
| `hotkey` | hotkey.rs | register, unregister |
| `terminal` | terminal.rs | spawn, write, kill |
| `mouse` | mouse.rs | move, click, scroll |
| `search` | search.rs | query, index, rebuild |
| `model` | model.rs | list, download, remove |
| `cmd` | builtin_cmd.rs | run built-in commands |
| `setting` | setting.rs | get, set |
| `calculator` | calculator.rs | eval |
| `workspace` | workspace.rs | list, switch, create |
| `note` | note.rs | list, get, save, delete |
| `history` | history.rs | list, clear |
| `ai` | ai.rs | chat, stream |
| `translation` | translation.rs | translate |
| `agent` | agent/mod.rs | run, cancel, status |
| `system_monitoring` | system_monitoring.rs | start, stop, snapshot |
| `nvim` | nvim.rs | detect, download |
| `automation` | automation.rs | execute |
| `plugin` | plugin.rs | list, load |

### 4.3 IpcError 格式

```json
{
  "code": "handler_error",
  "message": "human-readable description",
  "details": { "route": "...", "reason": "..." }
}
```

---

## 5. EventBus Topics

| Topic | 觸發時機 | payload 主要欄位 |
|-------|---------|----------------|
| `terminal.output` | terminal 有輸出 | `{ id, output }` |
| `config.reloaded` | 設定重新載入 | `{ source, changed_keys, changes }` |
| `config.reload_failed` | 設定載入失敗 | `{ source, code, error }` |
| `system.ping.completed` | ping 成功 | `{ message, registered_handlers }` |
| `ai.stream.chunk` | AI 串流回覆片段 | `{ session_id, chunk }` |
| `ai.stream.done` | AI 串流完成 | `{ session_id }` |
| `ai.stream.error` | AI 串流失敗 | `{ session_id, error }` |
| `agent.step` | ReAct agent 執行步驟 | `{ run_id, step, thought, action }` |
| `agent.done` | ReAct agent 完成 | `{ run_id, answer }` |
| `agent.error` | ReAct agent 失敗 | `{ run_id, error }` |
| `model.download.progress` | 模型下載進度 | `{ name, pct, stage }` |
| `model.download.done` | 模型下載完成 | `{ name, path }` |
| `model.download.error` | 模型下載失敗 | `{ name, error }` |
| `nvim-download-progress` | Neovim 下載進度 | `{ stage, pct, error? }` |
| `system_monitoring.snapshot` | 系統資源快照 | `{ cpu_pct, mem_mb, … }` |
| `translation.done` | 翻譯完成 | `{ text, target_lang }` |

> Tauri emit topic：將 `.` 替換為 `-`（`legacy_tauri_topic()`）。

---

## 6. 儲存設計

### 6.1 目錄佈局

| 路徑（Windows） | 用途 |
|---------------|------|
| `%APPDATA%\Roaming\Keynova\config.toml` | 使用者設定 |
| `%LOCALAPPDATA%\Keynova\knowledge.db` | SQLite（action log, agent memory, clipboard metadata） |
| `%LOCALAPPDATA%\Keynova\notes\` | Markdown 筆記檔案 |
| `%LOCALAPPDATA%\Keynova\search\tantivy\` | Tantivy 全文索引 |
| `%LOCALAPPDATA%\Keynova\nvim\` | Portable Neovim 執行檔 |

Linux/macOS 對應：`~/.config/keynova/` 與 `~/.local/share/keynova/`。

### 6.2 config.toml 主要結構

```toml
[hotkeys]
app_launcher = "Ctrl+K"
mouse_control = "Ctrl+Alt+M"

[terminal]
font_size = 13
scrollback_lines = 1000

[launcher]
max_results = 10

[mouse_control]
step_size = 15

[search]
backend = "tantivy"   # 或 "everything"（Windows）
index_dir = ""        # 空白使用預設路徑

[history]
max_items = 200

[notes]
storage_dir = ""      # 空白使用預設路徑
nvim_bin = ""         # 空白則 detect → portable 下載
```

### 6.3 knowledge.db Schema（版本 3）

| 資料表 | 主要欄位 | 用途 |
|-------|---------|------|
| `schema_version` | version INTEGER | schema 版本管理 |
| `action_log` | action_id, label, status, duration_ms, error | action 執行記錄 |
| `agent_audit` | run_id, event_type, status, summary, payload_json | ReAct audit trail |
| `agent_memory` | id, scope, workspace_id, title, content, visibility | Agent 長期記憶 |
| `clipboard_metadata` | item_id, content_type, workspace_id | 剪貼簿 metadata |

---

## 7. 搜尋架構

```
SearchHandler
    │
    ▼
SearchManager
    ├── AppManager      → 系統應用程式列表（OS API）
    ├── TantivyIndex    → 全文搜尋（tantivy crate）
    │     └── system_indexer → 掃描檔案系統建立索引
    ├── BuiltinCommandRegistry → 內建命令列表
    ├── NoteManager     → 筆記標題搜尋
    ├── HistoryManager  → 最近使用的 action
    └── WorkspaceManager → 工作區切換選項
```

**搜尋後端切換：**
- `backend = "tantivy"`（跨平台，預設）
- `backend = "everything"`（Windows 限定，使用 Everything SDK）

搜尋結果統一包裝為 `SearchResult { label, detail, action_ref, … }`，前端透過 `ActionRef` 執行後續動作。

---

## 8. Agent 架構（ReAct）

```
AgentHandler::execute("run", payload)
    │
    ▼
AgentRuntime::run(query, deps)
    │
    ▼
ReAct 迴圈（最多 N 步）：
    ├── Thought: LLM 產生推理
    ├── Action: 呼叫工具（filesystem, web, note, search…）
    │     handlers/agent/
    │       ├── intent.rs    → 解析 LLM 意圖
    │       ├── filesystem.rs → 檔案操作工具
    │       ├── web.rs       → Web 搜尋工具
    │       ├── formatting.rs → 輸出格式化
    │       └── safety.rs    → 工具權限 gate
    └── Observation: 工具結果 → 加入上下文
    │
    ▼
EventBus: agent.step / agent.done / agent.error
    │
    ▼
KnowledgeStore: agent_audit 寫入 SQLite
```

**AgentObservationPolicy** 控制哪些工具輸出可以被加入觀察（防止資訊洩漏）。

---

## 9. 擴展模式

新功能透過以下四個機制加入系統，**不修改 Core**：

| 機制 | 擴展方式 |
|------|---------|
| `CommandRouter` | 實作 `CommandHandler` trait，在 `app/state.rs` 呼叫 `command_router.register()` |
| `EventBus` | 呼叫 `event_bus.publish(AppEvent::new(topic, payload))`，前端 `listen()` 訂閱 |
| `ConfigManager` | 在 config.toml 新增 key，Handler 在初始化時讀取 |
| `SearchRegistry` | 實作 `SearchProvider` trait 並註冊至 `SearchManager` |
| `BuiltinCommandRegistry` | 實作 `BuiltinCommand` trait，呼叫 `reg.register(Box::new(...))` |
| `PanelRegistry.tsx` | 新增 `React.lazy` 映射，`CommandRouter` 回傳 `CommandUiType::Panel("name")` |

---

## 10. 平台差異

| 功能 | Windows | Linux | macOS |
|------|---------|-------|-------|
| 全域快捷鍵 | WinAPI | X11/Wayland | Accessibility API |
| 系統應用程式列表 | Registry + Start Menu | .desktop files | LaunchServices |
| 全文搜尋 | tantivy 或 Everything | tantivy | tantivy |
| 滑鼠控制 | WinAPI SendInput | xdotool | CGEvent |
| Neovim portable | nvim-win64.zip | nvim-linux64.tar.gz | nvim-macos.tar.gz |

平台特定程式碼透過 `#[cfg(target_os = "...")]` 隔離在 `platform/` 模組。

