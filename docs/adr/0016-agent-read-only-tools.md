# Agent Read-only Tool Runtime

**狀態：** 接受  
**日期：** 2026-05-05  
**決策者：** 開發者  
**相關文件：**
- docs/architecture.md
- docs/security.md

---

## 1. Context（技術背景）

`/ai` Agent 第一階段需明確限制 tool 範圍，避免在 approval/audit 機制尚未完善前開放寫入或 shell 能力。Agent 工具範圍過大會讓系統難以審計且風險難以評估。

## 2. Constraints（系統限制與邊界）

- ADR-022（approval boundary）尚未完整實作時，不可開放寫入工具
- ADR-027（sandbox）完成前，不可開放 shell 工具
- web.search 需使用者明確設定才啟用（隱私考量）
- tool 輸出需防止敏感資訊進入 LLM context

## 3. Alternatives Considered（替代方案分析）

### 方案 A：Read-only tools only（keynova.search + web.search）

**優點：**
- 風險範圍明確
- 無寫入或 shell 能力外洩風險

**缺點：**
- Agent 功能受限

### 方案 B：開放寫入與 shell tools

**優點：**
- Agent 功能完整

**缺點：**
- audit/approval 機制未完善時風險高
- 需先完成 ADR-022 + ADR-027

## 4. Decision（最終決策）

選擇：**方案 A — Read-only tools**

`agent.tool` 支援 `keynova.search`（本機搜尋 grounding）與 `web.search`（SearXNG JSON，預設 disabled）；不執行本機寫入或 shell。

原因：
- 第一階段先建立安全基礎，再逐步開放能力

犧牲：
- Agent 能力受限，某些使用場景需手動操作

Feature flag：`agent.web_search_provider`（預設 `disabled`）

Migration 需求：N/A

Rollback 需求：N/A

## 5. Consequences（系統影響與副作用）

### 正面影響
- `keynova.search` 安全讀取 workspace summary、command metadata、非敏感 settings schema、model catalog、notes/history 摘要
- `web.search` 只回 title/url/snippet，送出前拒絕 `private_architecture` / `secret` query term

### 負面影響 / 技術債
- `handlers/agent.rs` 注入 config/note/history/workspace/command/model context
- 寫入工具需等 ADR-022 + ADR-027 完成後解封

## 6. Implementation Plan（實作計畫）

已實作完成（Phase 4）。

## 7. Rollback Plan（回滾策略）

N/A — 限制更多工具不破壞既有功能。

## 8. Validation Plan（驗證方式）

| 測試類型 | 覆蓋目標 | 指令 |
|---------|---------|------|
| Security test | web.search query redaction | 手動 + 程式碼審查 |
| Unit test | tool registry 註冊 / 拒絕 | `cargo test -- agent` |

## 9. Open Questions（未解問題）

- 無（寫入工具解封條件見 ADR-022 / ADR-027）