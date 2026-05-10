# 開發 IDE 選擇

**狀態：** 接受  
**日期：** 2026-05-01  
**決策者：** 開發者  
**相關文件：**
- docs/architecture.md

---

## 1. Context（技術背景）

需選擇主要 IDE，確保 Rust + React 雙語言開發流程順暢。

## 2. Constraints（系統限制與邊界）

- 開發機：Windows 11
- 語言：Rust（主）+ TypeScript/React（次）
- 不提交任何 IDE 特定設定至 git

## 3. Alternatives Considered（替代方案分析）

### 方案 A：JetBrains RustRover

**優點：**
- 針對 Rust 最佳化，型別推斷與補全完整
- 整合 Clippy、cargo test 執行

**缺點：**
- 商業授權

### 方案 B：VS Code + rust-analyzer

**優點：**
- 免費，外掛豐富

**缺點：**
- rust-analyzer 在大型專案中記憶體佔用較高

## 4. Decision（最終決策）

選擇：**方案 A — JetBrains RustRover**

原因：
- Rust 開發體驗最佳
- 開發者熟悉度高

犧牲：
- 商業授權費用

Feature flag：N/A

Migration 需求：N/A

Rollback 需求：任何時候可切換 VS Code（`.idea/` 已在 `.gitignore`）

## 5. Consequences（系統影響與副作用）

### 對開發者的影響
- `.idea/`、`*.iml`、`*.iws`、`*.ipr` 全列入 `.gitignore`，不提交任何 IDE 設定

## 6. Implementation Plan（實作計畫）

已完成（`.gitignore` 設定）。

## 7. Rollback Plan（回滾策略）

切換 IDE 不影響程式碼，任何時候可換用 VS Code。

## 8. Validation Plan（驗證方式）

N/A — 工具選擇，不影響 runtime。

## 9. Open Questions（未解問題）

- 無