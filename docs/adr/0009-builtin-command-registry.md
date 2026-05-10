# BuiltinCommand 可擴展指令 Registry

**狀態：** 接受  
**日期：** 2026-05-02  
**決策者：** 開發者  
**相關文件：**
- docs/architecture.md

---

## 1. Context（技術背景）

需在不修改 core 的情況下動態新增內建指令（`/help`、`/setting`、`/note`、`/ai`、`/tr` 等），並供前端動態渲染指令清單。隨著 Phase 推進，指令數量持續增加，需要可擴展的 registry 機制。

## 2. Constraints（系統限制與邊界）

- 新增指令不得修改 `core/` 目錄
- 指令清單需透過 IPC 供前端查詢
- 前端 Panel 渲染需依指令結果動態決定（Inline 或 Panel 型別）

## 3. Alternatives Considered（替代方案分析）

### 方案 A：BuiltinCommand trait + Registry

**優點：**
- 新增指令 = 實作 trait + 注冊，不修改 core
- `cmd.list` IPC 供前端動態渲染
- 前端 PanelRegistry 以 `Record<string, ComponentType>` 映射

**缺點：**
- 需要多一個 trait 定義與 registry 結構

### 方案 B：硬編碼 match 分支

**優點：**
- 簡單

**缺點：**
- 每次新增指令需修改 core，違反開放封閉原則
- 難以測試個別指令

## 4. Decision（最終決策）

選擇：**方案 A — BuiltinCommand trait + Registry**

原因：
- 符合開放封閉原則，Core 不修改
- 前端動態渲染指令清單

犧牲：
- 新增一層抽象（BuiltinCommand trait）

Feature flag：N/A

Migration 需求：N/A

Rollback 需求：N/A

## 5. Consequences（系統影響與副作用）

### 正面影響
- 新增指令零 core 改動
- `cmd.list` IPC 供前端動態渲染完整指令清單
- 新增 `models/builtin_command.rs`（`CommandUiType::Inline|Panel`、`BuiltinCommandResult`）

### 對開發者的影響
- 前端 `PanelRegistry.tsx` 以 `Record<string, ComponentType>` 映射 panel name → React 元件，新增 panel 零前端 core 改動

## 6. Implementation Plan（實作計畫）

已實作完成（Phase 2）。

## 7. Rollback Plan（回滾策略）

N/A — 已完全實作，trait 定義向後相容。

## 8. Validation Plan（驗證方式）

| 測試類型 | 覆蓋目標 | 指令 |
|---------|---------|------|
| Unit test | BuiltinCommand trait / registry | `cargo test -- builtin_command_registry` |
| Integration test | cmd.list IPC | `cargo test -- cmd_dispatch` |

## 9. Open Questions（未解問題）

- 無