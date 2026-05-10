# Provider-Driven ReAct Agent Loop

**狀態：** 接受  
**日期：** 2026-05-08  
**決策者：** 開發者  
**相關文件：**
- docs/architecture.md
- docs/adr/0023-react-tool-schema-foundation.md
- docs/adr/0022-agent-approval-boundary.md

---

## 1. Context（技術背景）

Agent mode 需從 local prompt heuristics 升級為 provider-driven ReAct loop，讓 AI provider 決定是否回答或請求 tool calls。Local heuristics 能力受限，無法真正實現 tool-augmented reasoning。

## 2. Constraints（系統限制與邊界）

- ReAct loop 必須使用 user 選定的 AI provider/model，不強制走 API-only path
- local Ollama 必須是 first-class Agent backend
- Tool observations 只在通過 redaction/truncation 後才進入 LLM context
- 最大步驟數限制（防止無限迴圈）
- Rust 持有所有 policy / execution / audit 邏輯，不在 prompt 中

## 3. Alternatives Considered（替代方案分析）

### 方案 A：Provider-driven ReAct（LLM 決定 tool calls）

**優點：**
- AI provider 根據問題自主決定 tool 使用策略
- local heuristics 降為 offline fallback
- tool call 支援不同 provider（OpenAI tool call format / Ollama native）

**缺點：**
- provider 不支援 tool calls 時需 fallback
- 依賴 provider API 穩定性

### 方案 B：Local heuristics only

**優點：**
- 無 provider 依賴，完全離線

**缺點：**
- Agent 能力受限，無法真正 ReAct
- keyword matching 容易誤判

## 4. Decision（最終決策）

選擇：**方案 A — Provider-driven ReAct**

移至 provider-driven ReAct loop。Rust 持有 context filtering、tool schema generation、policy、approval、execution、observation redaction、cancellation、timeout、max-step limits 與 audit logging。

原因：
- AI provider 決定 tool 使用策略，能力遠超 local heuristics
- local Ollama 作為 first-class backend，不強制雲端

犧牲：
- provider 不支援 tool calls 時需 fallback 到 heuristics

Feature flag：`ai.provider` 設定（`ollama` / `openai-compatible`）

Migration 需款：N/A

Rollback 需款：切換回 heuristics（`ai.agent_mode = heuristic`）

## 5. Consequences（系統影響與副作用）

### 正面影響
- Provider adapters normalize model responses 為 final text 或 typed tool calls
- Rust 控制所有 policy，LLM 只做推理決策

### 負面影響 / 技術債
- Local prompt heuristics 成為 offline/fallback 行為，非主要 Agent planner
- Unsupported provider/tool-call 組合必須明確 fail 或 fallback 到 safe chat/heuristics
- **TD.4.A**：計畫將 approval polling 改為 async channel 以提升可測試性

### 對安全性的影響
- Rust 持有 execution / approval / audit logging，LLM 不直接執行 tool
- observation redaction 在 Rust 層執行，LLM 收到的已是 sanitized 輸出

## 6. Implementation Plan（實作計畫）

已實作完成（Phase 5）。

## 7. Rollback Plan（回滾策略）

設定 `ai.agent_mode = heuristic` 退回 local heuristics 模式。

## 8. Validation Plan（驗證方式）

| 測試類型 | 覆蓋目標 | 指令 |
|---------|---------|------|
| Unit test | ReAct step / thought / action | `cargo test -- agent_runtime` |
| Integration test | Ollama provider adapter | 需本機 Ollama 服務 |
| Security test | observation redaction in loop | `cargo test -- agent_observation` |

## 9. Open Questions（未解問題）

- [ ] TD.4.A：approval polling 改為 async channel（`AgentRunHandle` + cancel token）