# Structured Agent Web Search Providers

**狀態：** 接受  
**日期：** 2026-05-08  
**決策者：** 開發者  
**相關文件：**
- docs/architecture.md
- docs/security.md
- docs/adr/0016-agent-read-only-tools.md

---

## 1. Context（技術背景）

Agent web search 需要結構化 provider 以避免抓取完整 HTML 頁面。DuckDuckGo 不提供結構化 API，限於 best-effort fallback。未設定 provider 時，web search 應預設 disabled（隱私考量）。

## 2. Constraints（系統限制與邊界）

- web search 預設 disabled，需使用者明確設定才啟用
- 不得傳送使用者私人資料至外部搜尋引擎（query redaction）
- 所有 web results 需 normalize 為統一格式（title/url/snippet）
- Tavily API key 需設定於 ConfigManager（不暴露前端）

## 3. Alternatives Considered（替代方案分析）

### 方案 A：結構化 provider（SearXNG / Tavily）+ DuckDuckGo 作為 explicit fallback

**優點：**
- 結構化結果（title/url/snippet），不需 HTML 解析
- SearXNG 可自架，無第三方服務依賴
- Tavily 提供高品質 AI-optimized 結果

**缺點：**
- 需使用者設定 SearXNG URL 或 Tavily API key

### 方案 B：抓取 HTML 自行解析

**優點：**
- 無需外部 API

**缺點：**
- HTML 結構易變，維護成本高
- 抓取完整頁面資料量大，LLM context 浪費

## 4. Decision（最終決策）

選擇：**方案 A — 結構化 provider + DuckDuckGo 作為 explicit fallback**

Agent web search 預設 disabled，需配置結構化 provider 才啟用。SearXNG JSON 與 Tavily API 為結構化 adapter；DuckDuckGo HTML 僅作為 explicit best-effort fallback。

原因：
- 結構化結果品質好，LLM 更容易使用
- 預設 disabled 保護使用者隱私

犧牲：
- 需使用者設定才能使用 web search

Feature flag：`agent.web_search_provider`（`disabled` / `searxng` / `tavily` / `duckduckgo`）

Migration 需款：N/A

Rollback 需款：設定 `agent.web_search_provider = disabled` 停用

## 5. Consequences（系統影響與副作用）

### 正面影響
- 所有 web results normalize 為 grounded title/url/snippet
- 送出前通過 query redaction（拒絕 `private_architecture` / `secret` query term）

### 對安全性的影響
- Tavily API key 存於 ConfigManager，不暴露前端
- SearXNG URL 可自架（無第三方依賴）
- query redaction 防止私密資訊外洩

## 6. Implementation Plan（實作計畫）

已實作完成（Phase 5）。

## 7. Rollback Plan（回滾策略）

設定 `agent.web_search_provider = disabled` 停用 web search。

## 8. Validation Plan（驗證方式）

| 測試類型 | 覆蓋目標 | 指令 |
|---------|---------|------|
| Unit test | query redaction | `cargo test -- agent_web` |
| Security test | API key 不暴露前端 | 程式碼審查 |

## 9. Open Questions（未解問題）

- 無