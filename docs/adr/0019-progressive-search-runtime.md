# Progressive Search Runtime

**狀態：** 接受  
**日期：** 2026-05-06  
**決策者：** 開發者  
**相關文件：**
- docs/architecture.md

---

## 1. Context（技術背景）

第一批 UI 不應被 Everything/file backend 阻塞；需要前端可取消 stale search。搜尋結果應分層回傳：快速結果（app/command/note）先到，慢速結果（file/tantivy）後到。

## 2. Constraints（系統限制與邊界）

- 搜尋熱路徑延遲目標：快速 batch < 100ms，file batch < 500ms（有 timeout）
- 前端需可識別並丟棄過期搜尋結果（stale search）
- EventBus 用於推送後續 chunk 結果

## 3. Alternatives Considered（替代方案分析）

### 方案 A：同步相容路徑 + `stream=true` 選用

**優點：**
- 向後相容：不帶 `stream` 的呼叫行為不變
- stream 模式先回快速 batch，再推送慢速 chunk
- request id + backend generation 可讓前後端同時丟棄 stale search

**缺點：**
- 兩條路徑需同時維護

### 方案 B：純同步搜尋（等全部結果）

**優點：**
- 簡單

**缺點：**
- file backend timeout 會讓整個 `search.query` 等待，UI 凍結感

## 4. Decision（最終決策）

選擇：**方案 A — 同步相容路徑 + stream=true**

`search.query` 保留同步相容路徑，新增 `stream=true` 模式；stream 模式先回傳 app/command/note/history/model 的 quick batch，再由背景 worker 透過 `search.results.chunk` EventBus topic 發送後續結果。

原因：
- 漸進式 UX：使用者看到即時結果，不等候慢速 provider

犧牲：
- 兩條路徑需同時維護

Feature flag：`stream` 參數（payload 中的 bool）

Migration 需款：N/A

Rollback 需款：不帶 `stream=true` 即退回同步模式

## 5. Consequences（系統影響與副作用）

### 正面影響
- file provider 透過 timeout boundary 包起來；timeout 時回報 `timed_out_providers`
- `search.metadata` 成為 lazy metadata/preview 入口
- request id + backend generation 可讓前後端同時丟棄 stale search

### 負面影響 / 技術債
- **TD.4.B**：計畫進一步改成 SearchService actor（非同步 worker + cancel token）

## 6. Implementation Plan（實作計畫）

已實作完成（Phase 4）。

## 7. Rollback Plan（回滾策略）

不帶 `stream=true` 即退回同步模式，完全向後相容。

## 8. Validation Plan（驗證方式）

| 測試類型 | 覆蓋目標 | 指令 |
|---------|---------|------|
| Unit test | quick batch 路徑 | `cargo test -- search_handler` |
| Performance test | 快速 batch < 100ms | 手動量測 |

## 9. Open Questions（未解問題）

- [ ] TD.4.B：SearchService actor（async worker + cancel token）