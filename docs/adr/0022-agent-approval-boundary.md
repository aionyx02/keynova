# Agent Approval Boundary And Prompt Audit

**狀態：** 接受  
**日期：** 2026-05-07  
**決策者：** 開發者  
**相關文件：**
- docs/architecture.md
- docs/security.md
- docs/adr/0016-agent-read-only-tools.md

---

## 1. Context（技術背景）

Agent execution 需要明確的 approval state machine，且每次 run 需記錄 prompt audit 以確保透明度。無 approval 機制時，Agent 可能直接執行使用者未確認的高風險 action。

## 2. Constraints（系統限制與邊界）

- high-risk 與 medium-risk action 需不同 approval gate
- prompt audit 需記錄至 knowledge.db（可審計）
- approval polling 不可阻塞 UI（TD.4.A 計畫改為 async channel）
- Knowledge Store schema version 3 需備份

## 3. Alternatives Considered（替代方案分析）

### 方案 A：Explicit approval state machine + prompt audit

**優點：**
- 明確的 `waiting_approval` 狀態，UI 可顯示待確認 actions
- prompt audit 記錄 context budget / allowlisted sources / filtered sources
- Approved actions 回傳 UI-safe `BuiltinCommandResult`

**缺點：**
- approval polling 目前為輪詢（TD.4.A 計畫改為 async channel）

### 方案 B：所有 action 直接執行，只記錄 log

**優點：**
- 簡單

**缺點：**
- 使用者無法在執行前審查 high-risk action
- 安全風險不可接受

## 4. Decision（最終決策）

選擇：**方案 A — Explicit approval state machine + prompt audit**

Batch 4 agent execution 使用 explicit approval state machine 和 agent-owned planned actions。每個 run 記錄 prompt audit（context budget、allowlisted sources、filtered private/secret sources）。

原因：
- 使用者對 high-risk action 有明確確認機會
- prompt audit 提供透明度與可審計性

犧牲：
- 目前 approval polling 為輪詢，TD.4.A 計畫改善

Feature flag：N/A

Migration 需款：Knowledge Store schema version 3 新增 `agent_audit_logs` 和 `agent_memories`

Rollback 需款：降回 schema v2（刪除新資料表）

## 5. Consequences（系統影響與副作用）

### 正面影響
- `agent.start` 可回傳 `waiting_approval` runs，帶 typed planned actions
- Approved actions 回傳 UI-safe `BuiltinCommandResult`，AI panel 可開啟 note/setting/system/model panel 或 terminal
- Knowledge Store schema version 3 新增 `agent_audit_logs` 和 `agent_memories`；長期記憶為 opt-in

### 負面影響 / 技術債
- High-risk file writes 保持 scaffolded 而非直接執行
- **TD.4.A**：計畫將 approval polling 改為 async channel（`AgentRunHandle` + cancel token）

### 對安全性的影響
- prompt audit 記錄哪些 source 被 filtered（private/secret），確保敏感資訊不進入 LLM

## 6. Implementation Plan（實作計畫）

已實作完成（Phase 5）。

## 7. Rollback Plan（回滾策略）

- Knowledge Store schema 降回 v2：刪除 `agent_audit_logs` 和 `agent_memories` 資料表
- schema v3 升級前建立備份

## 8. Validation Plan（驗證方式）

| 測試類型 | 覆蓋目標 | 指令 |
|---------|---------|------|
| Unit test | approval state machine | `cargo test -- agent_approval` |
| Integration test | prompt audit 寫入 knowledge.db | `cargo test -- knowledge_store` |
| Security test | high-risk action gate | 手動 + 程式碼審查 |
| Migration test | schema v2→v3 | `cargo test -- knowledge_store_migration` |

## 9. Open Questions（未解問題）

- [ ] TD.4.A：approval polling 改為 async channel（`AgentRunHandle` + cancel token）