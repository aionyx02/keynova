# 後端與桌面框架

**狀態：** 接受  
**日期：** 2026-05-01  
**決策者：** 開發者  
**相關文件：**
- docs/architecture.md

---

## 1. Context（技術背景）

需選擇桌面 App 框架，目標包體積小、記憶體佔用低、安全性高。Keynova 需要系統 API 存取（全域快捷鍵、PTY、系統音量等），純 Web 框架無法滿足。

## 2. Constraints（系統限制與邊界）

- 平台：Windows 10+ / macOS 11+ / Linux (X11/Wayland)
- 包體積目標：~50MB
- 記憶體目標：冷啟動 < 120MB
- 啟動延遲：< 200ms（UI 可見）
- 需要存取系統 API

## 3. Alternatives Considered（替代方案分析）

### 方案 A：Rust + Tauri 2.x

**優點：**
- WebView2 (Windows) / WKWebView (macOS) 原生 WebView，打包體積 ~50MB
- 記憶體佔用遠低於 Electron
- Rust 型別安全、記憶體安全，無 GC 停頓
- 安全 IPC model（capability-based）
- 跨平台支援完整

**缺點：**
- Rust 學習曲線較高
- 各平台 WebView 渲染差異需個別處理

**效能分析：**
- 啟動時間：< 200ms（無 Chromium bundling）
- 包體積：~50MB
- 記憶體（冷啟動）：< 80MB

### 方案 B：Electron

**優點：**
- 生態最強，社群支援廣
- Node.js 後端熟悉度高

**缺點：**
- Chromium 打包體積 ~200MB
- 記憶體佔用高（~400MB）
- 不符合包體積目標

**效能分析：**
- 啟動時間：~1–3s
- 包體積：~200MB
- 記憶體：~400MB

## 4. Decision（最終決策）

選擇：**方案 A — Rust + Tauri 2.x**

原因：
- 符合包體積（~50MB）與記憶體（<120MB）目標
- Rust 提供記憶體安全，無 GC
- Tauri IPC 安全 capability model

犧牲：
- Rust 學習曲線
- 各平台 WebView 差異

Feature flag：N/A

Migration 需求：N/A（新專案）

Rollback 需求：N/A（新專案）

## 5. Consequences（系統影響與副作用）

### 正面影響
- 包體積、記憶體符合目標
- 後端邏輯以 Rust 撰寫，型別安全且效能高

### 負面影響 / 技術債
- Tauri 版本升級需關注 breaking change
- 跨平台需各自處理 Windows / macOS / Linux 差異

### 對安全性的影響
- Tauri IPC 層（`cmd_dispatch`）為前後端唯一通訊邊界
- capability-based 權限模型，前端無法直接存取系統 API

## 6. Implementation Plan（實作計畫）

已實作完成（Phase 1）。

## 7. Rollback Plan（回滾策略）

N/A — 已完全實作。

## 8. Validation Plan（驗證方式）

| 測試類型 | 覆蓋目標 | 指令 |
|---------|---------|------|
| Performance test | 啟動時間 / 記憶體 | 手動量測 Task Manager |
| Manual validation | 跨平台 UI | `npm run tauri dev` |

## 9. Open Questions（未解問題）

- 無