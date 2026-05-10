# Terminal Launch Specs For Builtin Commands

**狀態：** 接受  
**日期：** 2026-05-06  
**決策者：** 開發者  
**相關文件：**
- docs/architecture.md
- docs/adr/0009-builtin-command-registry.md

---

## 1. Context（技術背景）

`/note lazyvim` 需要直接啟動 nvim 透過 PTY，而非包在 default shell 中。目前 `BuiltinCommandResult` 只支援 `Inline` 與 `Panel` 型別，無法表達「啟動特定 program 於 PTY」的需求。

## 2. Constraints（系統限制與邊界）

- LazyVim 需要獨立的 config / data / state / cache 目錄（避免污染使用者的全局 nvim 設定）
- Editor session 中 Escape 鍵需保留給 vim，不能觸發 launcher 關閉
- nvim binary 需先透過 `detect_nvim()` 找到（ADR-001 依賴 portable nvim manager）

## 3. Alternatives Considered（替代方案分析）

### 方案 A：BuiltinCommandResult 支援 Terminal(TerminalLaunchSpec)

**優點：**
- 統一結果型別，前端 PanelRegistry 可處理
- launch_spec 攜帶完整資訊（program、args、cwd、env、title）
- plain terminal 仍走 pre-warmed shell path

**缺點：**
- BuiltinCommandResult 新增一個 variant

### 方案 B：獨立的 `/nvim` 指令（不走 BuiltinCommand）

**優點：**
- 不修改 BuiltinCommandResult 型別

**缺點：**
- 與其他 builtin command 不一致
- 重複前端 panel 管理邏輯

## 4. Decision（最終決策）

選擇：**方案 A — BuiltinCommandResult 支援 Terminal(TerminalLaunchSpec)**

`BuiltinCommandResult` 支援 `CommandUiType::Terminal(TerminalLaunchSpec)`。第一個消費者是 `/note lazyvim`，直接透過 PTY launch `nvim`。

原因：
- 統一結果型別，前端無需特殊處理
- launch_spec 完整描述 terminal 啟動方式

犧牲：
- BuiltinCommandResult 多一個 variant（維護成本低）

Feature flag：N/A

Migration 需求：N/A

Rollback 需款：移除 Terminal variant，回傳 Inline 錯誤訊息

## 5. Consequences（系統影響與副作用）

### 正面影響
- Terminal-backed command results 攜帶 `launch_id`、program、args、cwd、title、editor-session metadata
- `terminal.open` 接受可選 `launch_spec`；plain terminal 仍走 pre-warmed shell path

### 對使用者的影響
- Editor sessions 保留 Escape 給 vim，使用 `Ctrl+Shift+Q` 或 `Ctrl+Alt+Esc` 退出 launcher panel

### LazyVim 環境變數
- `NVIM_APPNAME`、`XDG_CONFIG_HOME`、`XDG_DATA_HOME`、`XDG_STATE_HOME`、`XDG_CACHE_HOME`（project-local `.keynova/lazyvim/`）

## 6. Implementation Plan（實作計畫）

已實作完成（Phase 4）。

## 7. Rollback Plan（回滾策略）

移除 Terminal variant，`/note lazyvim` 回傳 Inline 錯誤訊息。

## 8. Validation Plan（驗證方式）

| 測試類型 | 覆蓋目標 | 指令 |
|---------|---------|------|
| Integration test | PTY launch nvim | `cargo test -- terminal_launch_spec` |
| Manual validation | LazyVim 啟動 / Escape 行為 | `npm run tauri dev` |

## 9. Open Questions（未解問題）

- 無
