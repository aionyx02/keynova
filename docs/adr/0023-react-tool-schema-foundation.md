# ReAct Tool Schema And Observation Safety Foundation

**狀態：** 接受  
**日期：** 2026-05-08  
**決策者：** 開發者  
**相關文件：**
- docs/architecture.md
- docs/security.md
- docs/adr/0022-agent-approval-boundary.md
- docs/adr/0026-provider-driven-react-loop.md

---

## 1. Context（技術背景）

在接線 provider-driven ReAct loop 之前，需先確保 Agent tools 是 typed、schema-generated 且 observation-bounded，防止 unbounded stdout/search/file observations 進入 LLM context。未 bounded 的 observation 會導致 token overflow 或敏感資訊洩漏。

## 2. Constraints（系統限制與邊界）

- Tool parameter schemas 需從 Rust structs 自動生成（不手寫 JSON schema）
- Observation 必須通過 redaction/truncation pipeline，不能 bypass visibility policy
- OpenAI-compatible 與 local Ollama 為第一批 ReAct adapter 目標
- Generic shell 不在第一批 tool set（等 ADR-027 sandbox）

## 3. Alternatives Considered（替代方案分析）

### 方案 A：Typed foundation 優先（schema-generated + observation-bounded）

**優點：**
- 安全基礎明確，後續工具可快速加入
- observation redaction 統一處理，不需各工具自行處理
- `AgentToolSpec` 攜帶完整 metadata（approval policy、visibility policy、timeout、result limit）

**缺點：**
- 多一個 sprint 才能接 provider-driven loop

### 方案 B：直接接 ReAct loop（無 typed foundation）

**優點：**
- 開發快

**缺點：**
- observation unbounded，LLM 可能收到敏感內容
- 後續補 typed 成本高

## 4. Decision（最終決策）

選擇：**方案 A — Typed foundation 優先**

Batch 4.6 先建立 tool typed / schema-generated / observation-bounded foundation，再接 provider-driven ReAct loop。`AgentToolSpec` 攜帶 LLM-safe tool name、description、generated schemas、approval policy、visibility policy、timeout、result limit、observation redaction policy。

原因：
- 安全基礎優先，避免後期修補成本
- observation redaction 統一處理

犧牲：
- 多一個 sprint 才接 provider-driven loop

Feature flag：N/A

Migration 需款：N/A

Rollback 需款：N/A

## 5. Consequences（系統影響與副作用）

### 正面影響
- `AgentToolSpec` 統一描述所有工具的元資料
- observation redaction pipeline：likely secret lines 遮蔽、hard line/character cap、head+tail 保留

### 負面影響 / 技術債
- Generic shell execution 不在第一批 ReAct tool set（等 ADR-027 sandbox）
- `git.status` 等 typed tools 必須 approval-gated，不可透過 legacy `agent.tool` path 呼叫

### 對安全性的影響
- Tool observations 先通過 redaction，再進入 LLM context
- `AgentObservationPolicy` 控制哪些工具輸出可加入觀察

## 6. Implementation Plan（實作計畫）

已實作完成（Phase 5，Batch 4.6）。

## 7. Rollback Plan（回滾策略）

N/A — foundation layer，不改變 runtime 行為。

## 8. Validation Plan（驗證方式）

| 測試類型 | 覆蓋目標 | 指令 |
|---------|---------|------|
| Unit test | AgentToolSpec schema 生成 | `cargo test -- agent_tool` |
| Security test | observation redaction | `cargo test -- agent_observation` |

## 9. Open Questions（未解問題）

- 無