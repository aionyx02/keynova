# 終端方案

**狀態：** 接受  
**日期：** 2026-05-01  
**決策者：** 開發者  
**相關文件：**
- docs/architecture.md

---

## 1. Context（技術背景）

需在前端嵌入全功能終端，支援 ANSI escape code、PTY、跨平台。Keynova 的鍵盤工作流程需要即時 shell 互動能力。

## 2. Constraints（系統限制與邊界）

- 平台：Windows 10+ / macOS 11+ / Linux
- Windows 需支援 ConPTY（Windows Terminal 相同技術）
- PTY 讀取不可阻塞 UI thread
- PTY output 透過 EventBus 推送前端

## 3. Alternatives Considered（替代方案分析）

### 方案 A：xterm.js + portable-pty

**優點：**
- xterm.js：業界標準前端終端渲染器，支援完整 ANSI/VT100
- portable-pty：跨平台 PTY crate，Windows ConPTY 支援完整
- 社群維護活躍

**缺點：**
- xterm.js bundle 約 400KB（gzip 後 ~130KB）
- PTY 讀取需額外 thread

### 方案 B：自行實作終端

**優點：**
- 無外部依賴

**缺點：**
- 成本極高（ANSI parser、PTY abstraction、跨平台）
- 風險大，不在 MVP 範圍

## 4. Decision（最終決策）

選擇：**方案 A — xterm.js + portable-pty**

原因：
- 最成熟的跨平台方案
- Windows ConPTY 支援完整

犧牲：
- xterm.js bundle 體積

Feature flag：N/A

Migration 需求：N/A

Rollback 需求：N/A

## 5. Consequences（系統影響與副作用）

### 正面影響
- 前端 xterm.js 處理 rendering，後端 PTY 處理 I/O，職責分離清晰
- Windows 啟用 `windowsPty: { backend: "conpty" }`，與 Windows Terminal 體驗一致

### 負面影響 / 技術債
- PTY 讀取在獨立 thread，output 透過 EventBus 推送前端，增加一層事件

### 對安全性的影響
- PTY spawn 路徑需驗證 shell binary 路徑（防止 path injection）

## 6. Implementation Plan（實作計畫）

已實作完成（Phase 1）。

## 7. Rollback Plan（回滾策略）

N/A — 已完全實作。

## 8. Validation Plan（驗證方式）

| 測試類型 | 覆蓋目標 | 指令 |
|---------|---------|------|
| Integration test | PTY spawn / output / kill | `cargo test -- terminal` |
| Manual validation | ANSI color / resize / ConPTY | `npm run tauri dev` |

## 9. Open Questions（未解問題）

- 無