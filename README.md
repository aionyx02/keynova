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

### 已實作（v0.1）

#### 搜尋與啟動

| 快捷鍵 | 功能 | 說明 |
|--------|------|------|
| `Ctrl+K` | 開啟 / 關閉搜尋框 | 全域，任意前景視窗均可觸發；視窗已開啟時再按則關閉 |
| `↑` / `↓` | 在搜尋結果中移動選取 | 搜尋框內使用 |
| `Enter` | 啟動所選項目 | 應用程式直接開啟；檔案以系統預設程式開啟 |
| `Escape` | 關閉搜尋框 | App 常駐背景，不真正退出 |

#### 全域滑鼠控制

> 需先以 `Ctrl+Alt+M` 啟用滑鼠控制模式，其餘快捷鍵才會作用。

| 快捷鍵 | 功能 | 說明 |
|--------|------|------|
| `Ctrl+Alt+M` | 切換滑鼠控制模式 | 開啟 / 關閉，狀態變更會通知前端 UI |
| `Ctrl+Alt+W` | 游標向上移動 | 每次步進 15px（全域，不需要 Keynova 視窗聚焦） |
| `Ctrl+Alt+A` | 游標向左移動 | 同上 |
| `Ctrl+Alt+S` | 游標向下移動 | 同上 |
| `Ctrl+Alt+D` | 游標向右移動 | 同上 |
| `Ctrl+Alt+Enter` | 滑鼠左鍵點擊 | 點擊游標目前位置 |

---

### 規劃中

#### Phase 1（MVP v0.1）

| 快捷鍵 | 功能 | 說明 |
|--------|------|------|
| `Ctrl+Alt+T` | 開啟 / 關閉浮動終端 | 內建 PTY，支援多分頁；Phase 1.3 |

#### Phase 2（v1.0）

| 快捷鍵 | 功能 | 說明 |
|--------|------|------|
| `Ctrl+P` | 全域檔案搜尋 | Windows 優先使用 Everything IPC，否則 tantivy 索引 |
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
Presentation   → React 18 + TypeScript（CommandPalette、TerminalView）
IPC Bridge     → Tauri + EventBus（命令路由、事件推送）
Core           → CommandRouter / EventBus / ConfigManager / SearchRegistry
Business       → Handlers → Managers（launcher, hotkey, terminal, mouse, search）
Indexer        → tantivy（離線）/ Everything IPC（Windows 加速）
Platform       → #[cfg] 條件編譯隔離 Windows / Linux / macOS
```

## 開發命令

```bash
npm run lint        # ESLint
npm run build       # TypeScript + Vite 構建
cargo clippy -- -D warnings  # Rust lint
cargo test          # Rust 單元測試
```

---

**文件詳見 [`files/`](./files/) 目錄**
