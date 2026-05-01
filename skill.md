# skill.md — Codex 能力需求規範

> 本文件定義在 Keynova 專案中，對 Codex 的專業能力要求。
> 僅接受具備以下能力的輸出；不符合標準的程式碼請拒絕採用。

---

## 核心技術能力

### Rust（必要，高階）

- **所有權與借用**：能正確使用 `Arc<Mutex<T>>`、`Rc<RefCell<T>>`，並在必要時選擇正確的同步原語（`RwLock` vs `Mutex`）
- **非同步**：熟練使用 `tokio`，理解 `async/await`、`spawn`、`select!`、`broadcast` channel
- **Trait 設計**：能設計可擴展的 trait 介面（物件安全、泛型約束、`dyn Trait`），符合本專案 CommandRouter 的擴展模式
- **錯誤處理**：使用 `thiserror` / `anyhow` 建立清晰的錯誤層級，不使用 `.unwrap()` 於生產路徑
- **條件編譯**：正確使用 `#[cfg(target_os = "windows")]` 隔離平台專屬程式碼，不污染跨平台邏輯
- **效能意識**：理解零成本抽象、避免不必要的 clone，能識別 hot path 中的潛在瓶頸

### Tauri 2.x（必要，中高階）

- **IPC 命令**：使用 `#[tauri::command]` 定義命令，理解 `State<'_, T>` 的生命週期與執行緒安全要求
- **事件系統**：區分 `emit`（前端訂閱）與 `emit_to`（指定視窗），實作後端主動推送事件（EventBus）
- **視窗管理**：能建立浮動視窗、無邊框視窗、系統托盤，理解 `WebviewWindow` API
- **Plugin 系統**：理解 Tauri 2.x 的 plugin 架構，能在必要時封裝平台能力為 plugin
- **Capabilities 配置**：正確設定 `tauri.conf.json` 的權限（不授予超出需要的能力）

### TypeScript / React（必要，中階）

- **嚴格型別**：所有 Tauri IPC 呼叫需有對應的 TypeScript 型別定義，不使用 `any`
- **Zustand**：能設計扁平的 store 結構，使用 `immer` middleware 處理巢狀狀態，正確使用 selector 避免不必要 re-render
- **自訂 Hook**：將副作用（IPC 呼叫、事件訂閱）封裝為 hook，元件保持純粹
- **Tauri JS API**：熟悉 `@tauri-apps/api` 的 `invoke`、`listen`、`emit`，理解前後端事件訂閱的生命週期清理（`unlisten`）

---

## 專案專屬能力要求

### 架構理解

Codex 必須理解並遵守本專案的擴展模式：

```
新增功能 = 實作 Handler trait → 注冊至 CommandRouter
禁止直接修改 core/ 模組
```

不理解此模式的輸出將被拒絕。

### Windows 平台整合

- **Win32 API**：能透過 `winapi` / `windows-rs` crate 呼叫 Windows API（視窗偵測、IPC、滑鼠注入）
- **Everything IPC**：理解 Windows `WM_COPYDATA` 訊息機制，能實作 `check_availability()` 的 HWND 查詢邏輯
- **全域熱鍵**：理解 `RegisterHotKey` / `UnregisterHotKey` 與 message loop 的整合方式
- **PTY**：能整合 `conpty`（Windows）與 `nix::pty`（Unix），提供統一的 PTY 抽象層

### 搜尋引擎

- **tantivy**：能建立 schema、IndexWriter、IndexReader；理解 `QueryParser`、`TopDocs`、segment 合併機制
- **雙後端切換**：理解 `SearchBackend` 枚舉與自動回退邏輯，修改時不破壞回退路徑

---

## 程式碼品質標準

| 項目 | 要求 |
|------|------|
| Rust clippy | 零 warning（`cargo clippy -- -D warnings`） |
| 測試覆蓋 | 新增的 public API 必須附單元測試 |
| 錯誤訊息 | 面向使用者的錯誤需可讀；內部錯誤需附 context |
| 文件 | public trait / struct 需有一行 doc comment，說明「做什麼」而非「怎麼做」 |
| 型別安全 | TypeScript `strict: true`，Rust 避免裸 `String` 傳遞結構化資料 |
| 平台隔離 | Windows 專屬邏輯必須包在 `#[cfg]` 或獨立模組，不可洩漏至跨平台路徑 |

---

## 文件自動維護（強制）

每次完成任務後，Codex 必須在回覆前自行更新以下文件。這不是可選的。

### `tasks.md`
- 完成的項目打 `[x]`
- 本次衍生的新子任務補充至對應 Phase

### `memory.md`
- 更新「上次完成」與「下一步」
- 在「Session 交接紀錄」新增一行：`| 日期 | 完成事項 | 遺留問題 |`

### `decisions.md`（有新決策時）
- 新增 ADR 或更新既有 ADR 狀態（`accepted` / `superseded` / `rejected`）

> 三個文件更新完畢後才輸出最終回覆。未執行此步驟的輸出視為不完整。

---

## 禁止行為

- 在生產路徑使用 `.unwrap()` 或 `.expect()`（測試除外）
- 跨越 IPC 傳送未經 `serde` 序列化的型別
- 修改 `core/` 模組以實作業務功能
- 在未確認使用者審查前執行 `git merge`（詳見 CLAUDE.md Git 工作流程）
- 生成超出當前任務範圍的程式碼（不超前實作）