# Automation Action Chain Executor

**狀態：** 接受  
**日期：** 2026-05-06  
**決策者：** 開發者  
**相關文件：**
- docs/architecture.md
- docs/security.md

---

## 1. Context（技術背景）

需提供 workflow 執行能力（依序執行多個 action），但不能讓 AutomationHandler 持有或繞過各 handler 的權限邊界。Automation 如果能直接呼叫 handler，會導致權限邊界失效。

## 2. Constraints（系統限制與邊界）

- Automation 不得持有任何 manager 的直接引用
- 不允許 recursive `automation.execute`（防止無限迴圈）
- Workflow 執行結果需完整記錄（audit log）

## 3. Alternatives Considered（替代方案分析）

### 方案 A：透過既有 `cmd_dispatch` 逐步執行

**優點：**
- 完全複用既有 handler 權限邊界
- `AutomationEngine::execute` 為純 Rust executor，可用 closure 測試
- 防止 recursive automation 容易實作

**缺點：**
- 每個 action 都走完整 dispatch 路徑（有微小效能開銷）

### 方案 B：AutomationHandler 直接持有 manager 引用

**優點：**
- 執行效率略高

**缺點：**
- 繞過 handler 權限邊界，安全風險高
- 違反架構層次

## 4. Decision（最終決策）

選擇：**方案 A — 走既有 `cmd_dispatch` 邊界**

`automation.execute` 走既有 `cmd_dispatch` 邊界逐步執行 workflow actions，明確拒絕 recursive `automation.execute`。

原因：
- 最小化安全風險，完全複用既有邊界
- AutomationEngine 可獨立測試

犧牲：
- 每個 action 走完整 dispatch 路徑

Feature flag：N/A

Migration 需求：N/A

Rollback 需求：N/A

## 5. Consequences（系統影響與副作用）

### 正面影響
- `WorkflowExecutionReport` 回傳整體 log 與逐步 action execution
- `AutomationEngine::execute` 為純 Rust executor，可用 closure 測試

### 對安全性的影響
- Recursive automation 明確拒絕，防止無限迴圈與能力擴散

## 6. Implementation Plan（實作計畫）

已實作完成（Phase 4）。

## 7. Rollback Plan（回滾策略）

N/A — 純執行層，不影響資料格式。

## 8. Validation Plan（驗證方式）

| 測試類型 | 覆蓋目標 | 指令 |
|---------|---------|------|
| Unit test | AutomationEngine::execute | `cargo test -- automation` |
| Security test | recursive automation 拒絕 | `cargo test -- automation_recursive` |

## 9. Open Questions（未解問題）

- 無