# AI Provider 抽象化

**狀態：** 接受  
**日期：** 2026-05-03  
**決策者：** 開發者  
**相關文件：**
- docs/architecture.md
- docs/security.md

---

## 1. Context（技術背景）

需支援多個 AI provider（Ollama、OpenAI-compatible、未來 Claude API），並允許 local-first 操作。不同 provider 的 API 格式不同，需要統一抽象層。

## 2. Constraints（系統限制與邊界）

- API key 不得暴露於前端 bundle
- reqwest blocking 不可阻塞 tokio executor（需在 spawn_blocking 中執行）
- 多輪對話歷史保留最近 20 訊息，避免 token overflow
- local Ollama（port 11434）為預設 provider

## 3. Alternatives Considered（替代方案分析）

### 方案 A：Trait + provider adapters

**優點：**
- `AiProvider` trait 定義介面，各 provider 實作
- EventBus 非同步推送，前端 listen 事件
- 允許未來切換 Gemini / GPT / 本地模型

**缺點：**
- 多一層 trait 抽象，增加程式碼量

### 方案 B：硬編碼 Claude API

**優點：**
- 最快實作

**缺點：**
- 無法切換 provider，local-first 無法達成

## 4. Decision（最終決策）

選擇：**方案 A — Trait + provider adapters**

原因：
- local Ollama 為 local-first 核心需求
- 允許未來切換 provider 而無需修改 AI handler

犧牲：
- 多一層 trait 抽象

Feature flag：`ai.provider` 設定值

Migration 需求：N/A

Rollback 需求：改回特定 provider 實作

## 5. Consequences（系統影響與副作用）

### 正面影響
- API key 存於 ConfigManager（`ai.api_key`），不硬編碼
- 請求 ID 由前端產生，後端 event payload 帶回，前端對應 pending 狀態
- `AiProvider` / `TranslationProvider` trait 統一介面

### 負面影響 / 技術債
- timeout 預設 30 秒，可透過 `ai.timeout_secs` 設定（需記錄在文件）

### 對安全性的影響
- API key 存於 config.toml（明文），TD.5 計畫支援 OS keychain

## 6. Implementation Plan（實作計畫）

已實作完成（Phase 3）。

## 7. Rollback Plan（回滾策略）

N/A — trait 向後相容，新增 provider 不影響既有實作。

## 8. Validation Plan（驗證方式）

| 測試類型 | 覆蓋目標 | 指令 |
|---------|---------|------|
| Unit test | Provider trait mock | `cargo test -- ai_manager` |
| Integration test | Ollama HTTP 呼叫 | 需本機 Ollama 服務 |

## 9. Open Questions（未解問題）

- 無
