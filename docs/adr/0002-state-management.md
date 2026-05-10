# 前端狀態管理方案

**狀態：** 接受  
**日期：** 2026-05-01  
**決策者：** 開發者  
**相關文件：**
- docs/architecture.md

---

## 1. Context（技術背景）

需為前端選擇全域狀態管理方案，確保輕量且適合小中型桌面 App 快速迭代。CommandPalette、TerminalPanel、AiPanel 等元件需要共享狀態。

## 2. Constraints（系統限制與邊界）

- 框架：React 18（已決定，ADR-001）
- 不需要複雜的 middleware 或 time-travel debugging
- 桌面 App 規模（非大型多人協作前端）

## 3. Alternatives Considered（替代方案分析）

### 方案 A：Zustand

**優點：**
- 輕量，無 boilerplate，心智負擔低
- hooks-first API，與 React 18 完美整合
- 無 action/reducer 分離開銷

**缺點：**
- 需手動管理 slice 邊界，大型專案可能需拆多個 store

### 方案 B：Redux Toolkit

**優點：**
- 生態強，DevTools 完善
- 適合大型多人協作專案

**缺點：**
- 較重，boilerplate 多
- 對本 App 規模過度設計

### 方案 C：Jotai

**優點：**
- 原子模型，精細訂閱

**缺點：**
- 較少見，社群資源較少

## 4. Decision（最終決策）

選擇：**方案 A — Zustand**

原因：
- 輕量，Store 定義極簡
- hooks-first 符合 React 18 用法
- 對本 App 規模足夠

犧牲：
- 需手動管理 store slice 邊界

Feature flag：N/A

Migration 需求：N/A（新專案）

Rollback 需求：N/A（新專案）

## 5. Consequences（系統影響與副作用）

### 正面影響
- Store 定義 < 50 行，無 boilerplate
- 任何元件可直接訂閱所需 slice

### 負面影響 / 技術債
- 規模擴大時可能需要拆分 store

### 對開發者的影響
- State 存放在 `src/stores/`

### 對測試的影響
- Zustand store 可在測試中直接初始化，無需複雜 Provider 設定

## 6. Implementation Plan（實作計畫）

已實作完成（Phase 1）。

## 7. Rollback Plan（回滾策略）

N/A — 已完全實作。

## 8. Validation Plan（驗證方式）

| 測試類型 | 覆蓋目標 | 指令 |
|---------|---------|------|
| Unit test | Store action / selector | `npm run test` |

## 9. Open Questions（未解問題）

- 無