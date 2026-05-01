# Keynova - 全鍵控制系統

**完整項目計劃書 v2.0**

> 讓開發者從鍵盤開始，永遠不離開鍵盤。  
> A complete keyboard-centric productivity launcher for developers who never want to touch a mouse again.

---

## 📌 快速概覽

| 項目 | 說明 |
|------|------|
| **名稱** | Keynova - 全鍵控制系統 |
| **目標用戶** | 開發者、終端愛好者、生產力追求者 |
| **核心願景** | 95% 的工作流完全鍵盤操控，不需鼠標 |
| **目標平台** | Windows 10+ / Linux / macOS |
| **開發時長** | 6-8 個月（全職）或 12-16 個月（兼職）|
| **首個版本** | v1.0 MVP (12 週後) |
| **技術棧** | Tauri 2.x + React 18 + Rust |
| **開源模式** | MIT License（計劃開源）|
| **預計成本** | $100-200/年（可選雲服務）|

---

## 🎯 核心定位

### 解決的問題

現有應用（Alfred, Flow Launcher, Raycast）都是：
- ❌ 無法完全控制鼠標
- ❌ 無內置終端支持
- ❌ 無虛擬工作區系統
- ❌ 無專注模式集成

**Keynova 不同：**
- ✅ 全鍵盤操控 + 鼠標控制
- ✅ 集成 PTY 終端浮窗
- ✅ AI 助手（Claude）整合
- ✅ 虛擬工作區 + 浮窗管理
- ✅ 跨平台原生支持

---

## 📊 功能階段規劃

### 階段 1: MVP（第 1-6 週）✅ 核心啟動器

**目標：** 發佈可用的應用啟動器 + 終端浮窗 + 鍵盤鼠標控制

| 週次 | 任務 | 交付物 |
|------|------|--------|
| 1-2 | 項目搭建 + 框架初始化 | Tauri Hello World |
| 2-3 | 快速應用啟動器 (Win+K) | 應用掃描 + 模糊搜索 |
| 3-4 | 全局熱鍵系統 | 快捷鍵監聽 + 配置 |
| 4-5 | 終端浮窗 (Ctrl+Alt+T) | xterm.js + PTY |
| 5-6 | 鍵盤鼠標控制 | 方向鍵控制光標 |
| **發佈** | **v0.1 Alpha** | **可用但功能基礎** |

**成功指標：**
- ✅ 應用啟動 <100ms
- ✅ 終端無延遲
- ✅ 熱鍵無衝突
- ✅ Windows + Linux + macOS 都能運行

---

### 階段 2: v1.0 正式版（第 7-12 週）⚡ 實用工具整合

**目標：** 完整的生產力工具套件

| 功能 | 快捷鍵 | 詳細說明 |
|------|--------|---------|
| **快速計算器** | Ctrl+= | 單位轉換、日期計算 |
| **文件搜索** | Ctrl+P | 本地索引搜索 + Everything (Windows) |
| **剪貼簿管理** | Ctrl+Shift+V | 歷史記錄 + 圖片搜索 |
| **系統控制面板** | Win+S | 音量、亮度、WiFi、睡眠 |
| **命令執行** | Win+Shift+P | 直接執行系統命令 |
| **URL 浮窗預覽** | 自動識別 | 視頻/圖片/網頁內嵌 |

**技術重點：**
- Windows 文件搜索：優先使用 Everything IPC API，無則用 tantivy
- 跨平台：Linux/macOS 統一使用 tantivy 索引
- 剪貼簿：支持文本 + 圖片 + 歷史記錄

---

### 階段 3: v2.0 進階版（第 13-18 週）🚀 超越競品

**新功能：**

| 功能 | 說明 | 優先級 |
|------|------|--------|
| **AI 助手** | Ctrl+Shift+A 快速問 Claude | P0 |
| **專注模式** | 番茄工作法計時器 + 禁用分心應用 | P0 |
| **快速筆記** | Win+N 浮窗便簽（Markdown + Notion 同步） | P0 |
| **虛擬工作區** | Ctrl+Alt+1/2/3 預設應用組 | P1 |
| **浮窗管理** | 標籤頁 + 分割視圖 + 位置記憶 | P1 |
| **主題商店** | 社群提交的主題 + 自訂配色 | P2 |

**成功標記：**
- ✅ 開發工作流 100% 鍵盤
- ✅ 用戶日均使用 >6 小時
- ✅ GitHub Star >100
- ✅ 開源社群反饋正面

---

### 階段 4: v3.0+ 生態化（可選）🌿 插件系統

**外掛市場 + 雲端同步 + API 開放**

---

## 💻 技術架構概覽

### 技術棧確認

**前端：** React 18 + TypeScript + Tauri
**後端：** Rust + Tokio（非同步）
**終端：** xterm.js + PTY 管理
**搜索：** tantivy（主） + Everything IPC (Windows 可選)
**數據庫：** SQLite（歷史記錄）
**配置：** TOML 格式

### 6 層架構

```
┌─────────────────────────────────────────┐
│         演示層 (React UI)                 │
│  ┌─ CommandPalette  ┌─ Terminal          │
│  ├─ FloatingWindow  ├─ SystemPanel       │
│  └─ Config UI       └─ Notifications     │
├─────────────────────────────────────────┤
│    IPC 通訊層 (Tauri Bridge)              │
│  40+ RPC 命令接口                         │
├─────────────────────────────────────────┤
│    業務邏輯層 (6 個 Manager)              │
│  ┌─ HotkeyManager      ┌─ TerminalManager │
│  ├─ AppManager         ├─ MouseManager    │
│  ├─ SearchManager      └─ ConfigManager   │
│  └─ (涵蓋全部功能)                       │
├─────────────────────────────────────────┤
│    數據模型層 (Rust Structs)              │
│  ┌─ HotkeyConfig  ┌─ TerminalProcess    │
│  ├─ AppInfo       └─ SearchResult       │
│  └─ 結構化數據                          │
├─────────────────────────────────────────┤
│    平台適配層 (條件編譯)                  │
│  ┌─ Windows (RegisterHotKey)            │
│  ├─ Linux (X11/Wayland)                 │
│  └─ macOS (CGEventTap)                  │
├─────────────────────────────────────────┤
│    工具層 (Logger, Error, Utils)         │
│  日誌、錯誤處理、文件操作                │
└─────────────────────────────────────────┘
```

---

## 📈 時間與資源規劃

### 人力資源

**全職開發（推薦）**
```
1 人全職 × 6-8 個月 = 可交付 v1.0
成本：0（自研）或 2-3 萬人民幣（外包）
```

**兼職開發**
```
1 人業餘時間 × 12-16 個月 = 可交付 v1.0
成本：0
```

### 預算概估

| 項目 | 成本 | 說明 |
|------|------|------|
| 域名 (keynova.io) | $12/年 | 可選 |
| 服務器 (6 個月) | $30 | 插件市場（v3.0+） |
| Claude API | $60/年 | 可選，v2.0+ |
| 其他工具 | $0 | 全開源 |
| **總計** | **~$100/年** | **最小配置** |

---

## 🔄 工作流程

### Git 工作流

```bash
main        → 穩定版本（打 tag 發佈）
  ↑
develop     → 開發主分支（日常工作）
  ↑
feature/*   → 功能分支（新功能）
bugfix/*    → 修復分支（bug 修復）
```

### 提交規範

```
feat: add clipboard history with image support
fix: resolve hotkey conflict on Windows 11
docs: update README with installation guide
style: format code with prettier
refactor: extract search logic into separate module
test: add unit tests for HotkeyManager
chore: upgrade dependencies
```

### CI/CD 管道

```
push to GitHub
    ↓
GitHub Actions 觸發
    ↓
├─ Rust 單元測試 (cargo test)
├─ React 單元測試 (npm test)
├─ ESLint + Clippy 代碼檢查
└─ 多平台構建 (Win/Linux/macOS)
    ↓
全部通過 → 自動發佈 Release
```

---

## ⚠️ 風險與應對

### 高風險項

| 風險 | 概率 | 影響 | 應對方案 |
|------|------|------|---------|
| **跨平台兼容性** | 高 | 中 | 早期開始多平台測試 |
| **熱鍵衝突** | 高 | 中 | 完善衝突檢測 + 清晰提示 |
| **Everything 依賴** | 中 | 中 | 自訂索引作為備選 |
| **性能瓶頸** | 中 | 高 | 提前進行基準測試 |

### 應急計劃

```
進度延期 1 個月
  → 推遲 v2.0，先穩定 v1.0
  → 延長社群眾測期

無法實現某功能
  → 標記為「計劃功能」，納入 v3.0
  → 在發佈說明中明確說明

跨平台兼容失敗
  → 優先 Windows，其他平台延期
  → 發佈輕量級 CLI 版本作替代
```

---

## 📋 成功衡量

### 發佈後 3 個月的 KPI

| 指標 | 目標 | 衡量方法 |
|------|------|---------|
| **用戶下載** | 100+ | GitHub Release 統計 |
| **GitHub Star** | 50+ | Repository 頁面 |
| **活躍用戶** | 30+ | 遙測數據 |
| **反饋評分** | 80%+ 正面 | GitHub Issues + Discussions |

### 質量指標

| 指標 | 目標 |
|------|------|
| 測試覆蓋率 | >85% |
| 平均啟動時間 | <100ms |
| 終端延遲 | <50ms |
| Crash 率 | <0.1% |
| 內存占用 | <100MB (空閒) |

---

## 🚀 立即行動

### 第 1 週任務清單

- [ ] ✅ 確認項目名稱：**Keynova**
- [ ] ✅ 創建 GitHub 倉庫
- [ ] ✅ 初始化 Tauri + React 項目
- [ ] ✅ 搭建開發環境（Rust + Node.js）
- [ ] ✅ 編寫開發文檔
- [ ] ✅ 創建 Project Board（週計劃）

### 第 2-6 週任務

按照「階段 1: MVP」的週次規劃逐步推進

---

## 📚 文檔清單

| 文檔 | 位置 | 優先級 |
|------|------|--------|
| README.md | 項目根目錄 | P0 |
| DEVELOPMENT.md | /docs | P0 |
| ARCHITECTURE.md | /docs | P0 |
| API.md | /docs | P1 |
| PLUGIN.md | /docs | P2 |
| CHANGELOG.md | 項目根目錄 | P1 |
| USER_MANUAL.md | /docs | P1 |

---

## 💡 核心哲學

### 三個設計原則

1. **快速優先** - 所有操作 <200ms，否則失敗
2. **學習曲線平緩** - 新手 30 分鐘內掌握基礎
3. **完全可訂製** - 用戶可改所有快捷鍵、主題、插件

### 目標用戶週期

```
第 1 週：發現 Keynova，被 Demo 吸引
第 2-4 週：安裝 + 學習基礎快捷鍵
第 5-12 週：深度使用，開始自訂配置
第 3 個月+：成為常年使用者，社群貢獻
```

---

## 🎊 結語

**Keynova 不是又一個啟動器。**

它是一套完整的、以鍵盤為中心的生產力系統，為那些希望鍵盤成為唯一工具的開發者而設計。

讓我們一起把「打字的時刻就是思考的時刻」這一理念落實到極致。

---

**文檔版本：** v2.0  
**最後更新：** 2024-05-01  
**維護者：** Keynova 開發團隊

---

## 附錄：快速開始指令

```bash
# 克隆項目
git clone https://github.com/keynova/keynova.git
cd keynova

# 安裝依賴
npm install
cargo build

# 開發模式（Rust 熱重載）
npm run tauri dev

# 構建發佈版本
npm run tauri build

# 運行測試
npm test
cargo test
```

