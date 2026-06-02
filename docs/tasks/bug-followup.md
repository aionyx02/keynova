---
type: bug_followup
status: active
priority: p2
updated: 2026-06-02
context_policy: on_demand
owner: project
tags: [bug, crash, delete, diagnosis]
---

# Bug Follow-Up Plan — Launcher Crash + Delete No-Op

> Current routing note: this file is now background and regression context. The active P0 execution path is `docs/tasks/refactor-ai-capability.md`; Bug A/B checks are folded into `REF.2` and `REF.7`, and unrelated feature follow-ups stay parked until the refactor gate reopens.

> 兩個 critical bug 在 `feature/diagnostic-baseline-attempt` (9dfd15b) 上線了**診斷基建**，但**沒有真正解決使用者面感受的症狀**。本文件按優先序拆分後續工作，避免再陷入「猜+試」迴圈。

## 優先序

- **P0**：Bug A（閃退）— 使用者明說「閃退也很嚴重」。崩到完全用不下去，零容忍。
- **P0**：Bug B（假刪除）— data integrity 風險，使用者以為刪了但其實沒。

兩個並行，但 Bug A 先做（影響整個 app 可用性）。

---

## Bug A — Launcher 打字打到一半自動關掉（已找到根因）

### 症狀（2026-05-19 使用者澄清）

- **不是 renderer crash**：是 launcher window 自動 hide
- 使用者在搜尋框打字打到一半 → window 收起 → 需要重按 Ctrl+K 才能繼續
- 原本以為是閃退，實際是 **focus loss → auto-hide** 機制過於敏感

### 根因（已定位）

`src-tauri/src/app/window.rs:44-72` 監聽 `WindowEvent::Focused(false)` 並在 120ms 後 hide window。
觸發來源：

1. **Windows IME composition window**：中文 / 注音輸入法的 candidate window 短暫搶 keyboard focus → 主視窗收到 Focused(false) → 120ms 不夠 → hide
2. **WebView2 transparent window** 在 Windows 上的 accessibility subprocess 偶發 focus blip
3. **WebView2 popup / autocomplete** 等子視窗短暫拉焦

### 已落地修法（feature/diagnostic-baseline-attempt 後續）

- **Backend (`window.rs`)**：sleep `120ms → 400ms`，涵蓋絕大部分 IME / focus blip
- **Frontend (`CommandPalette.tsx`)** input element：加 `onCompositionStart` / `onCompositionUpdate` / `onCompositionEnd` 主動呼 `keepLauncherOpen()` 設 600ms guard。IME 期間絕對不會被 hide。
- 既有 `cmd_keep_launcher_open` IPC 沒動，guard 機制重用。

### 待驗證

- [ ] 使用者實測：中文打 keynova 注音、English fast typing 各 30 秒，確認不再 hide。
- [ ] 邊緣案例：如果 400ms 還是不夠（極慢 IME），考慮把 hide 改成需要連續兩次 sleep + recheck 都 unfocused 才執行。
- [ ] 若 IME 不是因，則改往原 R2 (WebView2 host crash) / R3 (Rust panic) 路線排查。

### 防呆基建（仍保留，跟主修法獨立）

- ErrorBoundary fallback card ✓
- Global error / unhandledrejection handler ✓ — 對 IME 修法無關，但對未來真崩潰仍有用

### 已知未明的事

1. 是 **renderer 內 React error** 還是 **WebView2 host 進程崩**？
   - React error → ErrorBoundary 應顯示 fallback card
   - WebView2 host 崩 → 整窗消失，跟使用者描述「閃退」一致
2. 觸發條件：開過 N 次後才崩？特定 query 後崩？workspace 切換時崩？
3. 是否與 Tauri global hotkey 重複註冊有關（`Ctrl+K` 在 `shortcuts.rs` 走 `on_shortcut` callback；fast toggle 可能 race）

### 步驟 1 — 證據蒐集（next session）

- [ ] **使用者操作**：跑 dev build 重現 1 次，撞到後：
  - 開 DevTools (F12)，看 Console 有沒有 `[keynova:window-error]` / `[keynova:unhandled-rejection]` 訊息 → 截圖貼回。
  - 看視窗是「黑屏 / 白屏」、「Reload 按鈕 card」、還是「視窗完全消失」 → 三種對應三條完全不同的因。
- [ ] 看 `%LOCALAPPDATA%\com.keynova.app\EBWebView\Default\` 下有沒有 `chrome_debug.log` / 任何 `*.dmp` minidump → 若有 WebView2 host crash 一定會留 minidump。
- [ ] 看 cargo run 啟動的 console（stderr）有沒有 Rust panic stack。

### 步驟 2 — 三條主嫌調查路線（依證據選一）

**路線 R1 — Renderer React error**（症狀：fallback card 出現 / console 有 keynova-prefix log）

- 看 ErrorBoundary 攔到的 stack
- 對焦 mount-time hooks：`useFilePreview`、`useSearchMetadata`、`hasCompletedOnboarding()`、`clearLegacyFilters()`、`useWindowResize`
- 修法：縮窄問題 hook 的 try/catch 或加 guard
- 工作量：小（~30 分鐘 per hook）

**路線 R2 — WebView2 host crash**（症狀：視窗完全消失 / EBWebView 下有 minidump）

- 可能因素：assetProtocol scope `**` + 某個 path 觸發 WebView2 bug、`tauri-plugin-opener` race、global shortcut callback 重入
- 修法：縮 assetProtocol scope、移除可疑 plugin call、考慮 Tauri 升級
- 工作量：大（可能需要 reproducer + Tauri issue tracker）

**路線 R3 — Tauri Rust panic**（症狀：cargo dev stderr 有 panic / app process 整個掉）

- 看 panic 位置：常見是 `unwrap()` on Mutex poisoning、async runtime panic 不被 catch
- 修法：對應位置加防呆
- 工作量：中

### 步驟 3 — 防呆 / 復原（即使修了主因也保留）

- [ ] 已落地：ErrorBoundary fallback card + global error handlers ✓
- [ ] 待加：Sentry-lite 本地 crash log writer (寫到 `%LOCALAPPDATA%\Keynova\crash.log`)，讓使用者下次回報能直接貼檔
- [ ] 待加：Tauri 側 `set_hook` 全域 panic handler，把 Rust panic 寫進同一個 crash.log

### 風險

- 沒固定 repro → 可能需要使用者每次撞到都貼一次 log，數次後才能歸納
- 若是 Tauri/WebView2 本體 bug → 修法可能需要 upstream patch 或 workaround

---

## Bug B — Delete is a no-op on disk

### 症狀

- 走 secondary action menu 的 Delete → Enter Enter
- UI 顯示 "Moved to recycle bin"、row 消失
- **檔案實際還在原磁碟路徑**
- Recycle Bin 也沒有該檔

### 已知

- 9dfd15b 加了 `verify_path_removed`，trash 假成功時會回 Err，UI hint 改顯示原因
- **但這不是真的解決**，只是把假成功變真錯誤；檔案還是沒刪掉

### 步驟 1 — 取得失敗訊息（需使用者配合）

- [ ] 使用者跑一次 Delete，看 hint 顯示什麼。預期顯示：「trash returned ok but file still exists at ... (Possible causes: file is open in another app, Recycle Bin disabled on this volume, insufficient permissions, or the path is on a network share without trash support)」
- [ ] 若 hint 是這樣 → 馬上問使用者：
  1. 測試檔案在哪個磁碟（C: / D: / 網路碟）？
  2. 該磁碟有 Recycle Bin 嗎？右鍵 desktop Recycle Bin → 內容 → 確認「啟用所有磁碟的回收筒」
  3. 檔案有沒有被其他程式開著？
  4. 使用者是否以 admin 跑 launcher？檔案需要 admin 才能動？
- [ ] 若 hint 不是這樣 → 表示 trash::delete 連 Ok 都沒回 → 看 error message

### 步驟 2 — 依根因選修法

**T1 — Recycle Bin 在該磁碟被禁用**：trash 行為對：reject 是合理的。修法：

- UI 改善：hint 直接告訴使用者「磁碟回收筒關了，請開啟或選永久刪除」
- 加 secondary action「Delete permanently (skip trash)」走 `std::fs::remove_file`（單獨 confirm，醒目紅色 UI）

**T2 — 檔案被其他程式 lock**：trash 行為對。修法：

- 加 retry：等 500ms 再試一次（搬到 backend 用 `tokio::time::sleep`）
- Hint 提示「請關閉開啟此檔的程式後重試」

**T3 — 權限不足**：trash 行為對。修法：

- Hint 提示「需要管理員權限」+ 建議啟用 elevated mode

**T4 — `trash` crate 本身有 bug**（最差情況）：

- 用 windows crate 直接呼 `IFileOperation` COM 替代 trash::delete
- 對 Linux/macOS 保留 trash crate（已知穩定）
- 加 cargo feature gate

### 步驟 3 — UI 改善（無論根因）

- [ ] hint 顯示 5s 太短可能還是讀不完 → 改成 hint 不自動消失，使用者按 Esc / 點 X 才關
- [ ] Delete failure 時把 row 上加紅框 + tooltip 顯示完整錯誤
- [ ] 加 secondary action「Delete permanently」(skip trash)，醒目紅色、需 2 段 confirm

### 風險

- 在使用者實際操作前無法確定根因（需要證據）
- IFileOperation 修法需 ~100 行 Windows COM 綁定，且 Tauri build 環境需 windows crate 對應 feature

---

## Bug A 與 B 的共同基建（已落地、不再動）

- ErrorBoundary fallback card：避免黑屏，給使用者重啟 / 複製錯誤的入口
- Global error handler：把 console.error 留下訊息
- Backend post-mutation verify：揭穿 trash 假成功
- localStorage filter persistence 移除：避免 UX 陷阱再現
- recentlyDeleted kill set：失敗時 row 還原來位置（refreshAfterFileMutation 只在 success path 呼）

---

## 執行順序建議

### 下一個 session（需使用者配合）

1. **(P0, ~20 min)** 使用者重現 Bug A 1 次 + Bug B 1 次，回報：
   - Bug A：DevTools console 截圖 / 視窗消失程度 / cargo stderr 內容
   - Bug B：hint 訊息文字 / 測試檔案路徑 / Recycle Bin 設定狀態
2. **(P0, ~30 min)** 我依證據選對應路線（R1/R2/R3 + T1/T2/T3/T4）
3. **(P0, 1-3 hr)** 實作修法 + 寫單測

### 後續 session

4. 部署 crash.log writer + Rust panic hook
5. 加「Delete permanently」secondary action（依 T1 結論決定優先序）

### 不在本批

- LAUNCH.2.C per-workspace quick actions
- ONBOARD.1.D/E
- UTIL.1.B online（ADR-038 待接受）
- NOTE.1 全部

## 關鍵 commit

- `9dfd15b feature/diagnostic-baseline-attempt` — 診斷基建上線
- `5eafe41 feature/launch-2-slice1` — LAUNCH.2 Slice 1（最後一個「正常運作」commit）

如果使用者要 rollback：`git checkout 5eafe41` 或 stash 模式（前次 conversation 已 demo）。
