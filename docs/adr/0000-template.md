# [決策標題，例如：使用 Tantivy 作為本機搜尋引擎]

**狀態：** 提議  
**日期：** YYYY-MM-DD  
**決策者：** 開發者 / AI agent / 團隊  
**相關文件：**
- docs/architecture.md
- docs/security.md
- docs/testing.md

---

## 1. Context（技術背景）

描述目前遇到的系統瓶頸、架構需求或維護問題。必須回答：

- 現在的問題是什麼？
- 為什麼現有設計不足？
- 目前的輸入規模大約是多少？
- 未來可能成長到什麼規模？
- 問題發生在 CPU、memory、disk I/O、network、UI responsiveness，還是 developer experience？
- 如果不處理，會產生什麼風險？

## 2. Constraints（系統限制與邊界）

列出平台、runtime、記憶體、啟動時間、資料量、安全、網路與背景任務限制。

- 平台：Windows 10+ / macOS 11+ / Linux (X11/Wayland)
- 包體積目標：~50MB
- 記憶體目標：冷啟動 < 120MB（目標 < 80MB）
- 啟動延遲：< 200ms（UI 可見）
- （其他限制依 ADR 內容填寫）

## 3. Alternatives Considered（替代方案分析）

至少列出兩個方案。若只有一個方案，也必須說明為什麼其他方案不可行。

### 方案 A：[方案名稱]

**優點：**
- ...

**缺點：**
- ...

**效能分析：**
- 時間複雜度：
- 空間複雜度：
- I/O 成本：
- 啟動成本：
- 長期維護成本：

**風險：**
- ...

### 方案 B：[方案名稱]

**優點：**
- ...

**缺點：**
- ...

**效能分析：**
- 時間複雜度：
- 空間複雜度：
- I/O 成本：
- 啟動成本：
- 長期維護成本：

**風險：**
- ...

## 4. Decision（最終決策）

說明建議選擇哪個方案、原因、犧牲、未來擴充空間、feature flag、migration 與 rollback 需求。

選擇：**方案 X**

原因：
- ...

犧牲：
- ...

Feature flag：（若無則填 N/A）

Migration 需求：（填寫 schema 變更、資料格式升級計畫）

Rollback 需求：（填寫如何退回）

## 5. Consequences（系統影響與副作用）

### 正面影響
- ...

### 負面影響 / 技術債
- ...

### 對使用者的影響
- ...

### 對開發者的影響
- ...

### 對測試的影響
- ...

### 對安全性的影響
- ...

## 6. Implementation Plan（實作計畫）

列出實作步驟，**此階段不得直接修改正式程式碼（ADR 未接受前）**。

1. ...
2. ...
3. ...

## 7. Rollback Plan（回滾策略）

說明如何退回原狀，包含：

- Feature flag 開關（若有）：
- 資料格式回滾：（刪除新欄位 / 降級 schema migration）
- 殘留檔案清理：
- 驗證退回是否成功：

## 8. Validation Plan（驗證方式）

包含單元測試、整合測試、效能測試、安全測試、邊界案例測試與手動驗證步驟。

若涉及效能，必須定義可量測指標，例如：
- 查詢延遲（ms）
- 啟動時間增加（ms）
- 記憶體增加（MB）
- N 筆資料下操作時間（ms）

| 測試類型 | 覆蓋目標 | 指令 / 步驟 |
|---------|---------|-----------|
| Unit test | 純函式、演算法 | `cargo test` / `npm run test` |
| Integration test | 多模組協作、I/O | `cargo test --test integration` |
| Regression test | 修復已知 bug | 同上 |
| Security test | 路徑、敏感資料 | 手動 + 程式碼審查 |
| Performance test | 搜尋、索引、批次 | 手動量測 Task Manager |
| Manual validation | 端到端 UX | 見步驟說明 |

## 9. Open Questions（未解問題）

列出尚未確定、需要開發者決策的問題。

- [ ] ...
- [ ] ...