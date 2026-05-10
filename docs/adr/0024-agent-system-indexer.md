# Agent SystemIndexer Search Path

**狀態：** 接受  
**日期：** 2026-05-08  
**決策者：** 開發者  
**相關文件：**
- docs/architecture.md
- docs/security.md
- docs/adr/0006-dual-search-backend.md

---

## 1. Context（技術背景）

Agent filesystem search 需要跨平台統一介面，並在各平台使用最佳索引器。直接遞迴掃描不符合效能目標，且在大型目錄下可能觸發 I/O 壓力。

## 2. Constraints（系統限制與邊界）

- 搜尋結果需包含 diagnostics（provider、fallback reason、visited count、permission-denied count、timeout、index age）
- fallback traversal 必須 bounded（visit count + timeout）
- 遵守 ignore filters（`.gitignore` 等）
- 非 Windows 平台需有 CLI indexer fallback

## 3. Alternatives Considered（替代方案分析）

### 方案 A：SystemIndexer 抽象層

**優點：**
- 各平台使用最佳工具：Windows Everything / macOS mdfind / Linux plocate/locate
- fallback：bounded parallel traversal（`ignore` crate）
- 統一介面，Agent 無需知道底層工具

**缺點：**
- 抽象層增加一定程式碼量

### 方案 B：直接遞迴掃描

**優點：**
- 簡單

**缺點：**
- 大型目錄（> 100K 檔案）I/O 壓力高
- 不使用系統索引器，速度慢

## 4. Decision（最終決策）

選擇：**方案 A — SystemIndexer 抽象層**

Agent filesystem search 透過 `SystemIndexer` 抽象層：
- Windows：Everything IPC → Tantivy
- macOS/Linux：固定 CLI indexer（`mdfind`、`plocate`、`locate`）透過 bounded stdout parsing
- Fallback：bounded parallel traversal（`ignore` crate）

原因：
- 各平台使用最佳工具，速度最快
- fallback 機制確保跨平台可用

犧牲：
- 各平台 CLI 工具可用性不同，需各自偵測

Feature flag：N/A

Migration 需款：N/A

Rollback 需款：N/A

## 5. Consequences（系統影響與副作用）

### 正面影響
- Agent filesystem results 包含 diagnostics，便於 debug
- Empty 或 missing CLI indexer results 不作為 final truth（fallback roots 可用時繼續嘗試）

### 對安全性的影響
- fallback traversal 遵守 ignore filters，bounded by visit count + timeout
- 不跟隨 symlink 逃出 workspace root

## 6. Implementation Plan（實作計畫）

已實作完成（Phase 5）。

## 7. Rollback Plan（回滾策略）

N/A — 平台特定偵測，不影響資料格式。

## 8. Validation Plan（驗證方式）

| 測試類型 | 覆蓋目標 | 指令 |
|---------|---------|------|
| Unit test | fallback traversal bounded | `cargo test -- system_indexer` |
| Integration test | Windows Everything IPC | 需本機 Everything 服務 |
| Performance test | 10K 檔案 search < 200ms | 手動量測 |

## 9. Open Questions（未解問題）

- 無