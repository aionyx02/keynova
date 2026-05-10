# App Module Split

**狀態：** 接受  
**日期：** 2026-05-06  
**決策者：** 開發者  
**相關文件：**
- docs/architecture.md

---

## 1. Context（技術背景）

`lib.rs` 同時承載 state、IPC dispatch、watchers、tray、shortcut、window、control plane，隨著 Phase 推進讓入口檔變得難以維護。單一檔案超過 1000 行，難以閱讀與測試。

## 2. Constraints（系統限制與邊界）

- `AppState` 必須在多個 app module 間共享（`pub(crate)`）
- Tauri command macro 必須在特定位置（compile-time 限制）
- 拆分不能改變任何 IPC contract 或 runtime 行為

## 3. Alternatives Considered（替代方案分析）

### 方案 A：拆分為 `app/` 子模組

**優點：**
- 職責分離：state / dispatch / bootstrap / watchers / shortcuts / tray / window
- 各模組獨立可測試
- `lib.rs` 保持 thin entrypoint

**缺點：**
- 需要一次性重構（但不改變行為）

### 方案 B：保留單一 `lib.rs`

**優點：**
- 不需重構

**缺點：**
- 長期維護成本高，難以閱讀

## 4. Decision（最終決策）

選擇：**方案 A — 拆分為 `app/` 子模組**

`src-tauri/src/lib.rs` 保持 thin entrypoint，將 app runtime 分成：
- `app/state.rs` — AppState 定義與初始化
- `app/bootstrap.rs` — Tauri 初始化與 command macro
- `app/dispatch.rs` — IPC 命令分派實作
- `app/control_server.rs` — 控制伺服器
- `app/watchers.rs` — EventBus relay loop
- `app/shortcuts.rs` — 全域快捷鍵設定
- `app/tray.rs` — 系統托盤
- `app/window.rs` — 視窗管理

原因：
- 長期可維護性
- 各模組職責清晰，可獨立測試

犧牲：
- 一次性重構工作量

Feature flag：N/A

Migration 需求：N/A（純模組重組，不改變行為）

Rollback 需款：N/A（模組合併回 lib.rs 即可）

## 5. Consequences（系統影響與副作用）

### 正面影響
- `AppState` 改為 `pub(crate)` app state，跨 app modules 共享
- Tauri command macro wrapper 保留在 `bootstrap.rs`，實際 command logic 委派 `dispatch.rs`

### 負面影響 / 技術債
- **TD.2**：下一步計畫進一步提取 `AppContainer` 與 `FeatureModule` trait，讓 `AppState::new()` 不直接 new 所有 manager

## 6. Implementation Plan（實作計畫）

已實作完成（Phase 4）。

## 7. Rollback Plan（回滾策略）

N/A — 純模組重組，行為不變。

## 8. Validation Plan（驗證方式）

| 測試類型 | 覆蓋目標 | 指令 |
|---------|---------|------|
| Integration test | 所有 IPC 路由仍可正常 dispatch | `cargo test` |
| Manual validation | 各功能正常運作 | `npm run tauri dev` |

## 9. Open Questions（未解問題）

- [ ] TD.2：`AppContainer` / `FeatureModule` trait 進一步抽象