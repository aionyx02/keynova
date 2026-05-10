# 文件搜尋雙後端策略

**狀態：** 接受  
**日期：** 2026-05-02  
**決策者：** 開發者  
**相關文件：**
- docs/architecture.md
- docs/security.md

---

## 1. Context（技術背景）

需提供跨平台檔案搜尋，同時允許 Windows 使用者享受 Everything 的即時搜尋體驗。Tantivy 需建立索引（啟動時或手動 rebuild），Everything 需已安裝且服務運行中。

## 2. Constraints（系統限制與邊界）

- 平台：Windows / macOS / Linux 均需可用
- Windows 使用者可能已安裝 Everything；非 Windows 沒有 Everything
- 搜尋後端切換不能影響 IPC contract（`SearchResult` 格式不變）
- Everything IPC 需 DLL 可用

## 3. Alternatives Considered（替代方案分析）

### 方案 A：Tantivy only

**優點：**
- 跨平台，無外部依賴
- 完全控制索引行為

**缺點：**
- Windows 不能享用 Everything 速度優勢
- 需要手動 rebuild index

### 方案 B：Everything only

**優點：**
- Windows 即時搜尋體驗極佳

**缺點：**
- Windows 專屬，非 Windows 無法使用

### 方案 C：雙後端（Tantivy + Everything IPC）

**優點：**
- 最大相容性：Windows 優先 Everything，失敗時自動 fallback Tantivy
- IPC contract 不變

**缺點：**
- 維護兩套 backend 程式碼
- Everything IPC 實作複雜度高

## 4. Decision（最終決策）

選擇：**方案 C — 雙後端（Tantivy + Everything IPC）**

原因：
- 跨平台相容性最佳
- Windows 使用者享有最佳體驗
- IPC contract 不變，frontend 無感知

犧牲：
- 需維護 `everything_ipc.rs`（目前為佔位符）

Feature flag：`[search].backend` 設定值（`auto` / `everything` / `tantivy` / `app_cache`）

Migration 需求：N/A

Rollback 需求：將 `search.backend` 改為 `tantivy` 即可強制使用 Tantivy

## 5. Consequences（系統影響與副作用）

### 正面影響
- Windows 使用者無需手動安裝額外工具即可享受快速搜尋
- Fallback 機制確保非 Windows 不中斷

### 負面影響 / 技術債
- `everything_ipc.rs` 目前為佔位符，完整實作尚需 Everything SDK 細節

### 對安全性的影響
- Everything IPC 使用 named pipe，需驗證 pipe server 身份

## 6. Implementation Plan（實作計畫）

已實作完成（Phase 2）。Everything IPC 完整實作待補（見 Open Questions）。

## 7. Rollback Plan（回滾策略）

- 設定 `search.backend = tantivy` 強制使用 Tantivy
- Everything IPC 失敗時自動 fallback，不需手動 rollback

## 8. Validation Plan（驗證方式）

| 測試類型 | 覆蓋目標 | 指令 |
|---------|---------|------|
| Unit test | Backend selection 邏輯 | `cargo test -- search_manager` |
| Integration test | Tantivy index / query | `cargo test -- tantivy` |
| Manual validation | Windows Everything fallback | `npm run tauri dev` |

## 9. Open Questions（未解問題）

- [ ] Everything IPC 完整實作需補官方 SDK 細節與錯誤處理策略（`everything_ipc.rs` 目前為佔位符）