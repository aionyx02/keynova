# Keynova: Industrial-Grade Desktop AI Orchestrator

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)]()
[![Release](https://img.shields.io/badge/release-v0.4.0--alpha-blue.svg)]()
[![Platform: Cross-platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux%20%7C%20macOS-lightgrey.svg)]()

## 🌟 專案簡介 (Description)

**Keynova** 不僅僅是一個啟動器，它是一個專為開發者與專業人士設計的 **桌面 AI 調度中樞**。在 AI 時代，我們需要的不再是單純的關鍵字搜尋，而是能理解作業系統狀態、具備長期記憶、且能在本機安全執行任務的「代理人（Agent）」。

Keynova 透過 Rust 的高效能核心，無縫銜接了本地模型（Ollama）與雲端 LLM，並建立了一套嚴格的 **Action Arena** 行為準則，讓 AI 可以在您的授權下操作檔案、控制滑鼠、甚至與第三方 API 互動，同時確保所有敏感資料（如原始碼架構、API Keys）受到本地隱私過濾器的嚴密保護。

## 🚀 核心技術亮點 (Deep Dive Features)

### 1. 雙引擎混合搜尋 (Hybrid Search Engine)
Keynova 實作了一套具備自動回退機制的搜尋架構：
- **Windows Everything IPC**: 在 Windows 上透過高效能 IPC 直接讀取 Everything 資料庫，達成 $O(1)$ 的秒級檔案索引。
- **Cross-platform Tantivy**: 在非 Windows 平台或作為補充時，使用 Rust 原生的 Tantivy 引擎建立全文本索引，支援複雜的模糊匹配與權重排序。
- **Action-Integrated Search**: 搜尋結果不僅是檔案，還包括可執行的 **ActionRef**（例如：直接在搜尋結果中預覽並執行 Git 指令）。

### 2. 本機優先的 AI Agent Runtime
- **Context Privacy Filter**: 自動過濾敏感資訊，防止私有專案架構或憑證被意外傳送至外部模型。
- **Grounding Sources**: Agent 可即時存取本機知識庫（Knowledge Store），包括您的筆記、歷史紀錄與剪貼簿上下文。
- **Streaming Tool Execution**: 支援流式輸出工具執行狀態，讓您能即時看見 Agent 正在搜尋哪些檔案或調用哪些 API。

### 3. Action Arena 安全隔離區
所有 AI 請求的系統操作都必須經過行為註冊系統：
- **風險等級評估**: 行為被分為 `Low` (讀取), `Medium` (網路存取), `High` (寫入/系統更改)。
- **Approval Gate**: 高風險操作強制觸發 UI 審核機制，確保 AI 不會在您不知情的情況下執行 `rm -rf`。

### 4. 模組化 Panel UI 系統
透過 `PanelRegistry` 實現了高度解耦的 UI 擴充：
- **/ai**: 多輪對話與 Agent 模式切換。
- **/tr**: 極速 Google 翻譯面板。
- **/note**: 基於 SQLite 的快速筆記本。
- **/history**: 全域動作審計與歷史紀錄查詢。

## 🛠️ 技術棧與架構 (Architecture)

Keynova 的底層架構旨在追求極致的穩定性與擴充性：

### 核心架構圖
```mermaid
graph TD
    subgraph Frontend (React & TypeScript)
        UI[Command Palette / Panels]
        Store[Zustand Store]
        IPC[useIPC Bridge]
    end

    subgraph Backend Core (Rust & Tauri)
        Router[Command Router]
        AA[Action Arena Registry]
        KS[Knowledge Store - SQLite]
        AS[Agent Runtime]
    end

    subgraph Platform Layer
        EV[Everything IPC - Windows]
        TV[Tantivy Engine]
        PTY[PTY / xterm.js]
    end

    UI --> IPC
    IPC --> Router
    Router --> AA
    AA --> KS
    AA --> AS
    AS --> TV
    AS --> EV
    Router --> PTY
```

## 📦 安裝與建置 (Installation)

### 開發者環境配置
1.  **安裝 Rust 工具鏈**: 確保 `cargo` 與 `rustc` 已安裝。
2.  **安裝 Node.js 依賴**:
    ```bash
    npm install
    ```
3.  **啟動本地 Ollama (選配)**:
    - 下載並安裝 [Ollama](https://ollama.com/)。
    - 推薦模型：`qwen2.5:7b` 或 `llama3.2:3b`。

### 執行與打包
```bash
# 開發模式
npm run tauri dev

# 建置產線版本 (Windows/macOS/Linux)
npm run tauri build
```

## ⌨️ 進階使用技巧 (Advanced Usage)

- **快速切換面板**: 輸入 `/` 即可喚起面板清單，例如 `/ai` 進入聊天，`/setting` 進入配置。
- **參數提示 (Args Hint)**: 在輸入指令時，面板底部會顯示即時的參數類型與說明。
- **終端預熱 (Terminal Pre-warm)**: Keynova 會在背景預先啟動 PTY session，確保您在切換到終端面板時能獲得即時的回饋。

## 📈 效能指標 (Performance)

| 指標 | 效能表現 | 備註 |
| :--- | :--- | :--- |
| **啟動響應時間** | $< 100ms$ | 從快捷鍵按下到面板完全顯示 |
| **檔案搜尋延遲** | $O(1)$ ~ $O(\log n)$ | 依賴 Windows Everything IPC 或 Tantivy |
| **記憶體佔用** | $\approx 45MB$ | 閒置狀態下的 Rust 核心佔用 |
| **IPC 往返延遲** | $< 1ms$ | 透過 Tauri 非同步橋接實現 |

## 🗺️ 開發路線圖 (Roadmap)

- [x] **Phase 1-3**: 核心啟動器、搜尋引擎與基本 Panels。
- [x] **Phase 4 Foundation**: Action Arena 與 Knowledge Store 基礎。
- [ ] **Phase 4.5**: 完善 Agent Mode 的真實 Web/File 工具呼叫。
- [ ] **Phase 5**: WASM 插件熱插拔與社群插件商店。
- [ ] **Mobile Sync**: 與行動端筆記同步功能。

## 🤝 參與貢獻 (Contributing)

Keynova 是一個開放的專案，我們非常歡迎各種形式的貢獻：
1. **Bug Report**: 如果您發現任何不符合預期的行為。
2. **Feature Request**: 告訴我們您希望 Agent 具備什麼樣的「超能力」。
3. **Documentation**: 幫助我們完善技術手冊。

請閱讀 `memory.md` 以了解目前的技術邊界與開發規範。

## 📜 授權 (License)

本專案採用 **MIT License**。

---

**維護者**: Shawn & Keynova Contributors
**專案位置**: [GitHub - Keynova](https://github.com/aionyx02/keynova)
