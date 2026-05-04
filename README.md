# Keynova

> 鍵盤優先的開發者生產力啟動器 — 90%+ 工作流無需滑鼠

基於 **Tauri 2.x + React 18 + Rust** 構建，輕量 ~50MB，跨平台（Windows / Linux / macOS）。

---

## 快速開始

```bash
npm install
npm run tauri dev   # 開發模式（熱重載）
npm run tauri build # 生產構建
```

---

## ⌨️ 快捷鍵速查

### 已實作（Phase 2.A / 2.B）

#### 搜尋、終端、指令

| 快捷鍵 / 前綴 | 功能 | 說明 |
|--------|------|------|
| `Ctrl+K` | 開啟 / 關閉搜尋框 | 全域，任意前景視窗均可觸發 |
| 無前綴 | 搜尋模式 | 模糊搜尋 App + 檔案（Everything 優先，否則 fallback 掃描 C / D / WSL） |
| `>` 前綴 | 終端模式 | 視窗展開為 PTY 終端（xterm.js + pre-warm），支援 PowerShell / cmd |
| `/` 前綴 | 指令模式 | 顯示內建指令建議，`/help` 列出所有指令、`/setting` 開啟設定面板 |
| `↑` / `↓` | 移動選取 | 搜尋結果或指令建議中導航 |
| `Enter` | 執行所選項目 | 開啟 App / 檔案，或執行指令 |
| `Escape` | 關閉 / 退出終端 | App 常駐背景，不真正退出 |

#### 全域滑鼠控制

> 需先以 `Ctrl+Alt+M` 啟用滑鼠控制模式，其餘快捷鍵才會作用。

| 快捷鍵 | 功能 | 說明 |
|--------|------|------|
| `Ctrl+Alt+M` | 切換滑鼠控制模式 | 開啟 / 關閉，狀態變更會通知前端 UI |
| `Ctrl+Alt+W` | 游標向上移動 | 每次步進 15px（全域） |
| `Ctrl+Alt+A` | 游標向左移動 | 同上 |
| `Ctrl+Alt+S` | 游標向下移動 | 同上 |
| `Ctrl+Alt+D` | 游標向右移動 | 同上 |
| `Ctrl+Alt+Enter` | 滑鼠左鍵點擊 | 點擊游標目前位置 |

---

### 規劃中

#### Phase 2 剩餘（v1.0）

| 快捷鍵 | 功能 | 說明 |
|--------|------|------|
| `Ctrl+=` | 快速計算機 | 支援單位轉換、進位換算 |
| `Ctrl+Shift+V` | 剪貼簿歷史 | 文字 + 圖片預覽，快速貼上 |
| `Win+S` | 系統控制面板 | 音量、亮度、WiFi 快速調整 |

#### Phase 3（v2.0）

| 快捷鍵 | 功能 | 說明 |
|--------|------|------|
| `Ctrl+Alt+1` / `2` / `3` | 切換虛擬工作區 | 組織視窗與應用，快速切換情境 |
| `Ctrl+Shift+A` | AI 助理 | 整合 Claude API；程式碼解釋、文件生成、快速提問 |
| `Win+N` | 快速筆記 | Markdown 編輯 + Notion 同步 |

---

## 架構

```
Presentation   → React 18 + TypeScript（CommandPalette、TerminalPanel、SettingPanel）
IPC Bridge     → Tauri + EventBus（cmd_dispatch 路由、terminal-output 事件推送）
Core           → CommandRouter / EventBus / ConfigManager / BuiltinCommandRegistry
Business       → Handlers → Managers（launcher, hotkey, terminal, mouse, search, builtin_cmd, setting）
Indexer        → Everything IPC（Windows 加速）/ fallback 掃描（C/D/WSL home）/ tantivy（規劃中）
Platform       → #[cfg] 條件編譯隔離 Windows / Linux / macOS
```

### 搜尋優先順序（Windows）

```
1. Everything IPC  ← 已安裝 Everything 且服務運行時
2. Fallback 掃描   ← Desktop / Downloads / Documents / Pictures /
                      D:–Z: 槽 / \\wsl.localhost\<Distro>\home
```

### 擴展指令

新增 `/command` 只需：

```rust
// 1. 實作 trait
struct MyCmd;
impl BuiltinCommand for MyCmd {
    fn name(&self) -> &'static str { "mycmd" }
    fn description(&self) -> &'static str { "做某件事" }
    fn execute(&self, _args: &str) -> BuiltinCommandResult { ... }
}

// 2. 在 lib.rs 注冊
reg.register(Box::new(MyCmd));
// 前端自動顯示於 / 建議列表，零改動
```

---

## 開發命令

```bash
npm run lint                  # ESLint
npm run build                 # TypeScript + Vite 構建
cargo clippy -- -D warnings   # Rust lint
cargo test                    # Rust 單元測試
npm run tauri dev             # 完整開發環境（熱重載）
```

---

**文件詳見 [`files/`](./files/) 目錄**
 
## Keynova CLI

The Tauri package also builds a small local-control binary named `keynova`.
It talks to the running app over `127.0.0.1` JSON control messages and does not
use any remote branch or external network service.

```bash
keynova start   # Launch Keynova if needed, or focus the running launcher
keynova down    # Gracefully quit the running app
keynova reload  # Reload %APPDATA%\Keynova\config.toml and apply runtime settings
keynova status  # Check whether the app control plane is alive
```

If `keynova` is not recognized in your shell during development, use:

```bash
npm run keynova -- start
npm run keynova -- status
```

To make `keynova` available globally in PowerShell/CMD, install it once:

```bash
cargo install --path src-tauri --bin keynova --force
```

Runtime config changes flow through the same reload pipeline whether they come
from `/setting key value`, the settings panel, `keynova reload`, `/reload`, or
an external edit to `%APPDATA%\Keynova\config.toml`. Hotkeys are re-registered
in place; launcher, terminal, and mouse settings are refreshed without a full
app restart.

---

### Reload Scope

| Setting area | Runtime behavior |
| --- | --- |
| `hotkeys.*` | Re-registers global shortcuts in place |
| `launcher.max_results` | Refreshes the command palette search limit |
| `terminal.font_size`, `terminal.scrollback_lines` | Updates open terminal views and applies to new sessions |
| `mouse_control.step_size` | Applies on the next keyboard mouse movement |

If `keynova reload` fails, check that the app is running with `keynova status`
and validate `%APPDATA%\Keynova\config.toml` as TOML. The app emits
`config-reload-failed` to the settings panel and writes the detailed error to
stderr.
