# 樣式方案

**狀態：** 接受  
**日期：** 2026-05-01  
**決策者：** 開發者  
**相關文件：**
- docs/architecture.md

---

## 1. Context（技術背景）

需要能快速建置一致 UI 的樣式方案，減少命名與設計 token 管理成本。Keynova UI 以深色主題、鍵盤操作為主，需要細緻的 spacing / color token 控制。

## 2. Constraints（系統限制與邊界）

- 框架：React 18 + TypeScript（ADR-001）
- 桌面 App，不需 SSR
- 需支援深色主題

## 3. Alternatives Considered（替代方案分析）

### 方案 A：Tailwind CSS

**優點：**
- utility-first，快速迭代
- 設計 token（color/spacing）統一於 tailwind.config
- 不引入運行時 JS 成本

**缺點：**
- className 字串較長
- 需靠 IDE 自動補全

### 方案 B：CSS Modules

**優點：**
- 樣式隔離性好

**缺點：**
- class 命名成本高，設計 token 需額外管理

### 方案 C：styled-components

**優點：**
- JS-in-CSS，動態樣式容易

**缺點：**
- 運行時成本高，需要 Babel plugin

## 4. Decision（最終決策）

選擇：**方案 A — Tailwind CSS**

原因：
- 快速迭代，無運行時成本
- 設計 token 集中管理

犧牲：
- className 字串較長，需 IDE 支援

Feature flag：N/A

Migration 需求：N/A（新專案）

Rollback 需求：N/A（新專案）

## 5. Consequences（系統影響與副作用）

### 正面影響
- 主題色 / spacing token 集中於 tailwind.config，改動方便
- 無運行時 CSS-in-JS 開銷

### 負面影響 / 技術債
- className 可讀性較低，需靠工具協助

## 6. Implementation Plan（實作計畫）

已實作完成（Phase 1）。

## 7. Rollback Plan（回滾策略）

N/A — 已完全實作。

## 8. Validation Plan（驗證方式）

| 測試類型 | 覆蓋目標 | 指令 |
|---------|---------|------|
| Manual validation | 視覺一致性 | `npm run tauri dev` |

## 9. Open Questions（未解問題）

- 無