# Terminal PTY Pre-warm 策略

**狀態：** 接受  
**日期：** 2026-05-03  
**決策者：** 開發者  
**相關文件：**
- docs/architecture.md
- docs/testing.md

---

## 1. Context（技術背景）

Windows shell 同步 spawn 需 200–800ms，導致終端初始延遲 > 1 秒，影響使用者體驗。Keynova 鍵盤工作流程要求終端幾乎即時可用（< 50ms）。

## 2. Constraints（系統限制與邊界）

- Windows 預設 shell：PowerShell / pwsh（可用 `KEYNOVA_TERMINAL_SHELL` 環境變數覆寫）
- PTY spawn 為 blocking 操作，不可在 Mutex 鎖內執行（否則會 deadlock）
- WSL 需繼承 `TERM=xterm-256color` / `COLORTERM=truecolor` 等 capability hints
- Pre-warm session 需在 App 啟動時背景初始化，不阻塞 UI

## 3. Alternatives Considered（替代方案分析）

### 方案 A：每次按需 spawn

**優點：**
- 簡單，無狀態管理

**缺點：**
- 延遲高（Windows > 200ms），使用者感受到明顯卡頓

**效能分析：**
- 啟動時間：200–800ms（每次）
- 記憶體：無額外佔用

### 方案 B：Pre-warm 一個 shell

**優點：**
- 啟動時背景 spawn，claim 後補充下一個，第一次開啟幾乎零延遲（< 50ms）

**缺點：**
- 額外記憶體佔用（一個閒置 shell process）
- 狀態管理複雜度增加

**效能分析：**
- 第一次 open 延遲：< 50ms（claim pre-warmed session）
- 記憶體：額外 ~20–50MB（一個閒置 shell）

## 4. Decision（最終決策）

選擇：**方案 B — Pre-warm**

原因：
- 使用者體驗關鍵路徑：終端延遲直接影響工作流程
- 記憶體代價可接受

犧牲：
- 額外一個閒置 shell process 的記憶體

Feature flag：N/A（可考慮日後加入 `terminal.prewarm = false` 設定）

Migration 需求：N/A

Rollback 需求：移除 `warm: Option<WarmEntry>` 欄位，改回每次 spawn

## 5. Consequences（系統影響與副作用）

### 正面影響
- 終端首次開啟 < 50ms（Windows）
- 使用者感受流暢鍵盤工作流程

### 負面影響 / 技術債
- `TerminalManager` 新增 `warm: Option<WarmEntry>` 欄位，狀態複雜度略增
- Claim warm session 後必須立即補充下一個

### 對 API 的影響
- `terminal.open` 回傳型別從 `String` 改為 `{ id, initial_output }`
- `terminal.open` 接收前端 xterm rows/cols，claim warm session 後立即 resize

### 對使用者的影響
- `AppState` 新增短時間 focus guard，避免 ESC 從 terminal 回 search 時被 WebView2 blur-to-hide 誤關閉

### 對安全性的影響
- blocking 的 `openpty + spawn_command` 必須在 `Mutex<TerminalManager>` 鎖外執行，防止 deadlock

## 6. Implementation Plan（實作計畫）

已實作完成（Phase 1）。

## 7. Rollback Plan（回滾策略）

移除 `warm: Option<WarmEntry>` 欄位，`terminal.open` 改回同步 spawn。

## 8. Validation Plan（驗證方式）

| 測試類型 | 覆蓋目標 | 指令 |
|---------|---------|------|
| Integration test | Pre-warm claim 流程 | `cargo test -- terminal_prewarm` |
| Performance test | 首次開啟延遲 < 50ms | 手動量測 |
| Manual validation | Windows ConPTY + pre-warm | `npm run tauri dev` |

## 9. Open Questions（未解問題）

- 無