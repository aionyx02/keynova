# 前端框架選擇

**狀態：** 接受  
**日期：** 2026-05-01  
**決策者：** 開發者  
**相關文件：**
- docs/architecture.md

---

## 1. Context（技術背景）

需為 Tauri 桌面 App 選擇前端框架，確保生態成熟且開發效率高。Keynova 需要豐富的 UI 元件、hooks 機制與 TypeScript 型別安全。

## 2. Constraints（系統限制與邊界）

- 平台：Windows 10+ / macOS 11+ / Linux (X11/Wayland)
- 包體積目標：~50MB（含 Tauri shell）
- 需與 Tauri 2.x WebView bridge 相容
- 開發者熟悉度

## 3. Alternatives Considered（替代方案分析）

### 方案 A：React 18 + TypeScript

**優點：**
- 生態最成熟，Tauri 官方範本直接支援
- TypeScript 保證型別安全
- 大量 ecosystem（hooks、zustand、xterm.js）可直接使用
- 社群與第三方 UI library 豐富

**缺點：**
- Virtual DOM 略有運行時開銷（對此 App 規模可忽略）
- 需維護 TypeScript 型別定義與 tsconfig

### 方案 B：Vue 3

**優點：**
- Composition API 設計優雅
- 官方工具鏈完善

**缺點：**
- 開發者熟悉度較低
- Tauri 官方範本支援不如 React

### 方案 C：Svelte

**優點：**
- 無 Virtual DOM，compile-time 框架，包體積小

**缺點：**
- 生態與第三方 UI library 較少
- Tauri 整合範本較少

## 4. Decision（最終決策）

選擇：**方案 A — React 18 + TypeScript**

原因：
- Tauri 官方範本直接支援，零額外設定
- TypeScript 保證 IPC payload 型別安全
- 生態豐富，xterm.js、zustand 等直接可用

犧牲：
- Virtual DOM 運行時微小開銷（對本專案規模無影響）

Feature flag：N/A

Migration 需求：N/A（新專案）

Rollback 需求：N/A（新專案）

## 5. Consequences（系統影響與副作用）

### 正面影響
- Tauri 整合零額外設定
- TypeScript 在前後端 IPC 邊界提供型別安全

### 負面影響 / 技術債
- 需維護 tsconfig、ESLint 設定

### 對使用者的影響
- 無（基礎設施決策）

### 對開發者的影響
- 所有前端元件使用 `.tsx` 副檔名
- 需遵守 TypeScript strict mode

### 對測試的影響
- 使用 Vitest 或 React Testing Library 進行前端測試

### 對安全性的影響
- TypeScript 型別安全減少前端 payload injection 風險

## 6. Implementation Plan（實作計畫）

已實作完成（Phase 1）。

## 7. Rollback Plan（回滾策略）

N/A — 基礎框架決策，已完全實作。

## 8. Validation Plan（驗證方式）

| 測試類型 | 覆蓋目標 | 指令 |
|---------|---------|------|
| Unit test | React 元件行為 | `npm run test` |
| Type check | TypeScript 型別 | `npx tsc --noEmit` |
| Manual validation | UI 操作流程 | `npm run tauri dev` |

## 9. Open Questions（未解問題）

- 無