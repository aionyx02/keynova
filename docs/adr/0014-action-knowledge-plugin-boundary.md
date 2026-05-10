# Phase 4 Action / Knowledge / Plugin 邊界

**狀態：** 接受  
**日期：** 2026-05-05  
**決策者：** 開發者  
**相關文件：**
- docs/architecture.md
- docs/security.md

---

## 1. Context（技術背景）

隨著 Agent 與 Plugin 加入，需明確定義能力邊界，防止高風險能力擴散。搜尋熱路徑不應跨 IPC 傳送肥大的 JSON action payload；SQLite 不可進入 Tauri command/search hot path。

## 2. Constraints（系統限制與邊界）

- 搜尋熱路徑延遲目標：< 100ms
- SQLite 不可在 UI 同步路徑執行
- Agent 與 Plugin 若直接取得 shell/filesystem/network/AI key 會形成高風險能力擴散
- KnowledgeStore 必須支援 App shutdown 前 flush

## 3. Alternatives Considered（替代方案分析）

### 方案 A：ActionArena + KnowledgeStore + Plugin capability deny-by-default

**優點：**
- 前端只接收輕量 `ActionRef`，完整 payload 留在 Rust
- SQLite 透過 dedicated worker thread，不阻塞熱路徑
- Plugin 預設 deny 高風險能力

**缺點：**
- 多一層間接層（ActionRef 解析）

### 方案 B：直接傳送完整 action payload

**優點：**
- 簡單

**缺點：**
- 搜尋結果 JSON 體積大，IPC 傳輸成本高
- 敏感 action 資訊暴露於前端

## 4. Decision（最終決策）

選擇：**方案 A — ActionArena + KnowledgeStore + Plugin deny-by-default**

原因：
- 搜尋熱路徑保持輕量
- 高風險能力明確隔離

犧牲：
- ActionRef 生命周期管理增加複雜度（session_id + generation）

Feature flag：N/A

Migration 需求：N/A

Rollback 需求：N/A

## 5. Consequences（系統影響與副作用）

### 正面影響
- 新增 `core/action_registry.rs`（ActionRef + ActionArena）
- 新增 `core/knowledge_store.rs`（rusqlite + bounded channel + WAL）
- Knowledge Store write path 支援 action/clipboard batch insert

### 負面影響 / 技術債
- shutdown 前先 flush DB worker（`schedule_shutdown` 中的 `knowledge_store.flush().await`）
- `SettingPanel` 改由 schema 驅動，敏感設定遮蔽顯示

### 對安全性的影響
- Agent 與 Plugin Runtime 預設 deny 高風險能力
- 所有本機動作必須走 Action System 或 Host Functions，不可直接取得 shell/AI key

## 6. Implementation Plan（實作計畫）

已實作完成（Phase 4）。

## 7. Rollback Plan（回滾策略）

ActionArena 為 in-memory 結構，重啟後清空；KnowledgeStore flush 失敗時 WAL 保護資料完整性。

## 8. Validation Plan（驗證方式）

| 測試類型 | 覆蓋目標 | 指令 |
|---------|---------|------|
| Unit test | ActionArena insert / resolve | `cargo test -- action_registry` |
| Integration test | KnowledgeStore write / flush | `cargo test -- knowledge_store` |
| Security test | Plugin deny-by-default | 手動 + 程式碼審查 |

## 9. Open Questions（未解問題）

- 無