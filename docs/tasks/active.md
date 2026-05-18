---
type: task_index
status: active
priority: p0
updated: 2026-05-19
context_policy: always_retrievable
owner: project
tags: [feature-first, safety-first, performance, agent-ux]
last_change: Bug A 真根因定位 — window auto-hide 對 IME composition focus blip 太敏感; 120ms→400ms + frontend onComposition* 撐住 guard
---

# Active Tasks

## Feasibility Verdict (2026-05-14)

- Feasibility is high for low-memory runtime plus safe local review if rollout is staged.
- FEAT.11 runtime work remains blocked until ADR, security policy, and typed DTO prerequisites are complete.
- RAM goal is scoped to Background Core only (exclude active WebView, loaded LLM model, PTY terminal session, and monitoring streams).

## Strategy

Safety first for runtime lifecycle; feature-first delivery after guardrails are in place.

## Current Execution Order

Mainline delivered (詳見 `docs/tasks/completed.md`)：

- TD.5.A baseline → PERF.1 → TD.1/2/3 → PERF.2 → PERF.3 → TD.4 + TD.5 → P3 → FEAT.11 → Phase 7a (AGENT.1/2/7) → LAUNCH.1.A + LAUNCH.1.B → LAUNCH.1.C/D/E → UTIL.1 offline parts → **UTIL.2**.

Active queue：**empty**（UTIL.2 全段已完成；ONBOARD.1、LAUNCH.2、NOTE.1 仍可平行起手，皆無 ADR 阻擋。UTIL.1.B-online 待 ADR-038、其他 ADR-029 ~ 037 仍 pending）。

## Next Phase Proposal (2026-05-15, awaiting selection)

Source: agent UX + 功能實用性 + keynova 整體 baseline 缺口盤點。

Recommended parallel entry tracks (highest ROI, no ADR blockers):

- **Phase 7a — AGENT.1 / AGENT.2 / AGENT.7**：streaming、markdown、cancel、approval UX。立即改善等待感與訊息可操作性。
- **Phase 8a — UTIL.1 / UTIL.2**：calculator++ 與 dev utilities。純 local、可平行於 agent 改造。
- **Phase 8b — LAUNCH.1**：search 結果 secondary actions（reveal / copy path / file ops）。放大既有 search 投資。
- **Phase 8c — ONBOARD.1**：first-run tour、`?` cheatsheet、empty-state CTA。降低學習曲線。

ADR-gated tracks（需等 ADR 接受才能進實作，可先草擬 ADR）：

- AGENT.3 (ADR-029) — agent 工具集從「research helper」改為「action executor」
- AGENT.4 (ADR-030) — OS-level selection capture
- CLIP.1 (ADR-031) — clipboard history
- SNIP.1 (ADR-032) — text expansion
- WIN.1 (ADR-033) — window switcher
- UTIL.3 (ADR-034) — scheduler / reminder
- DEV.1.C/D (ADR-035) — external provider auth
- SYNC.1.D/E/F (ADR-036) — git-backed sync
- AI.1 (ADR-037) — inline AI surfaces

See `docs/tasks/backlog.md` Post-FEAT.11 Phase Proposal 與 11 個 track section（總計約 80 個子任務）。
See `docs/tasks/blocked.md` Post-FEAT.11 Tracks Pending ADR 表。

## Recent Execution Notes

- 2026-05-19: **Bugfix — Bug A 真根因：IME composition focus blip 觸發 auto-hide**
  - 使用者澄清「閃退」實情：搜尋框打字打到一半 window 自己收起，需重按 Ctrl+K — **不是 renderer crash，是 launcher window auto-hide**。
  - 定位：`src-tauri/src/app/window.rs:44-72` 對 `WindowEvent::Focused(false)` 等 120ms 後 hide。Windows IME composition window（中文 / 注音輸入法）短暫搶 keyboard focus 就觸發；WebView2 transparent window 的 accessibility subprocess 也有同樣 blip。120ms 太短。
  - 修法（2 處小改）：
    - Backend (`window.rs:48`)：`120ms → 400ms`，涵蓋絕大部分 IME / focus blip。
    - Frontend (`CommandPalette.tsx` input)：新增 `onCompositionStart` / `onCompositionUpdate` / `onCompositionEnd` 主動呼 `keepLauncherOpen()` IPC 設 600ms guard。IME composition 期間絕對不會被 hide。
  - 既有 `cmd_keep_launcher_open` IPC 沒動，guard 機制重用。
  - 檢查：`cargo check` 清；`cargo clippy -- -D warnings` 清（先前 9dfd15b verifier tests 還在）；`npx tsc --noEmit` 清、`npm run lint` 清。
  - 待使用者實測：中文打字、English fast typing 各 30 秒，確認不再 hide。
  - Bug B（假刪除）狀態不變：9dfd15b 的 verify_path_removed 已就位，但 trash silent fail 的根因仍需使用者 repro + hint 訊息來判定（T1/T2/T3/T4 路線見 `docs/tasks/bug-followup.md`）。

- 2026-05-18: **Bugfix — launcher crash + delete-without-delete (two critical bugs)**
  - Symptoms reported by user: (1) Ctrl+K opens launcher but webview intermittently dies with no log trail; (2) destructive Delete via secondary action menu shows "Moved to recycle bin", hides the row from results, but the file **remains on disk at its original path**.
  - Root causes:
    - **Bug 1**: no top-level React `ErrorBoundary`, no global `window.addEventListener('error' | 'unhandledrejection')`. Any uncaught error in CommandPalette mount-time hooks tore down the entire WebView with no recovery and no console evidence.
    - **Bug 2**: `handlers/file.rs::"delete"` arm called `trash::delete()` and trusted its `Ok()` return. `trash` v5 wraps `SHFileOperationW` on Windows, which is documented to return success without moving the file in several edge cases (file in use, Recycle Bin disabled on the target volume, network shares without trash, permission quirks). Frontend's `recentlyDeleted` kill-set then hid the row, producing a "fake delete" experience.
  - **Slice 1 — backend post-mutation verification (`src-tauri/src/handlers/file.rs`)**: new `verify_path_removed(target)` checks `fs::symlink_metadata(target)` after `trash::delete` and returns `Err` (with a long diagnostic listing likely causes) when the file still resolves at its original path. `"rename"` and `"move"` arms call a parallel `verify_rename_landed(source, target)` checking that source is gone AND target exists. Both helpers are pure and unit-tested (4 new tests). Backend `Err` propagates through dispatch → frontend `catch` → `showHint` (duration bumped 3000→5000 ms so the user can read the longer message). The row stays in the result list because `refreshAfterFileMutation` is only invoked on the success branch.
  - **Slice 2 — defensive frontend baseline**: new `src/components/ErrorBoundary.tsx` (top-level class component with `getDerivedStateFromError` + `componentDidCatch`; renders an actionable fallback card with `Reload` + `Copy error` buttons; no IPC dependency so it survives even when providers crash). `AppContainer` wraps `CommandPalette` in `<ErrorBoundary>`. `src/main.tsx` registers `window.addEventListener('error' | 'unhandledrejection')` that `console.error` with `[keynova:window-error]` / `[keynova:unhandled-rejection]` tags — when the user hits the crash again the WebView leveldb log will retain the trace.
  - **Checks**: `cargo test handlers::file` 26/26 + 1 ignored (+4 new verifier tests); full suite 350/351 (pre-existing `note_lazyvim_missing_nvim_returns_inline_guidance` unchanged); `cargo clippy -- -D warnings` clean; `npm run lint` clean; `npx tsc --noEmit` clean.
  - **Pending user verification before commit** (per user request "測試過後再修正"):
    1. Bug 2 lock case: open `test.txt` in Notepad (locks file) → search & Delete in launcher → expected: hint shows "trash returned ok but file still exists at ... (possible causes: …)", row stays in list, file remains on disk. Earlier behavior would silently hide the row.
    2. Bug 2 happy case: regular file delete → row vanishes, file moved to Recycle Bin, verify in Explorer.
    3. Bug 1: open/close launcher 20× via Ctrl+K/Esc. If crash recurs, DevTools console (F12) should now hold `[keynova:*]` tagged lines — paste those back.
    4. If a render error fires, ErrorBoundary fallback card should show with `Reload` button — confirm it's not a blank window.

- 2026-05-18: **Bugfix — secondary action menu clipped before Delete**
  - Symptom: short result panes clipped the action popover, so lower actions such as `Delete` were not visible.
  - Fix: `SecondaryActionMenu` now uses a fixed `390 x 360` panel with a scrollable action body and automatic `scrollIntoView` for keyboard focus. `CommandPalette` reserves a stable left-column height while the menu is open and allows the overlay to remain visible instead of being clipped by the result card.
  - Checks: `npx tsc --noEmit` passes; `npm run lint` passes. Vite browser smoke page starts at `http://127.0.0.1:5173/`; full Tauri-backed APP_CACHE interaction requires the desktop runtime.

- 2026-05-18: **Bugfix — stale deleted files still appearing from APP_CACHE / persisted indexes**
  - Root cause: the previous deleted-path guard only covered file operations initiated inside Keynova. Files deleted externally could remain in APP_CACHE, Tantivy, or native indexer results until rebuild.
  - Fix: `SearchManager::file_results_for_backend` now over-fetches candidates, drops file/folder results whose `metadata` no longer exists or whose kind no longer matches, then applies the display limit. This covers APP_CACHE, Tantivy, Everything, and non-Windows native indexer paths.
  - Tests: added two focused regression tests for stale-path removal and limit-after-filter behavior; `cargo test --manifest-path src-tauri/Cargo.toml filter_existing_file_results` passes; `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings` passes. Full `cargo test --manifest-path src-tauri/Cargo.toml` remains 346 passed / 1 failed / 1 ignored with the known pre-existing `note_lazyvim_missing_nvim_returns_inline_guidance` failure.

- 2026-05-18: **Bugfix — LAUNCH.1.D filter chips silently hiding results across sessions**
  - 症狀：使用者搜尋任何字串時，file/folder 結果全消失。
  - 診斷：實際讀 `%LOCALAPPDATA%\com.keynova.app\EBWebView\Default\Local Storage\leveldb` 發現 `keynova.searchFilters` 殘留為 `["note","app"]`。`visibleResults` derivation 根據此 set 把 file/folder 全濾掉。
  - 根因：LAUNCH.1.D 設計把 chip selection 持久化到 localStorage（原 spec 寫 "URL state 同步"，桌面 app 沒 URL 我換成 localStorage）。用戶在某次 toggle 後切 chip 沒清回，跨 session 卡住所有後續搜尋。
  - 修正 (`src/components/FilterChips.tsx`)：`loadFilters()` 改為永遠回空 Set；新 `clearLegacyFilters()` 主動 `localStorage.removeItem` 舊鍵；`saveFilters` 整段移除。`CommandPalette.tsx` mount 時 `useEffect(clearLegacyFilters, [])` 清掉殘留；移除 saveFilters auto-write effect。Filter chips 改為 session-scoped — 重啟 app 即清空，避免「卡住的 filter」陷阱。
  - 檢查：`npx tsc --noEmit` 清、`npm run lint` 清。

- 2026-05-18: **Bugfix — LAUNCH.1.B destructive ops leak deleted entries**
  - 症狀：使用者對 file 執行 `delete` / `rename` / `move` 並 confirm 後，原項目仍出現在 result list；再按 Enter 開啟會 hit dead path、再按 → 開菜單可能對錯誤對象操作。
  - 第一輪修正（不足）：optimistic local removal + 透過 `handleQueryChange(query)` re-fire search。問題：Windows Everything 索引 Recycle Bin entries，re-fire 會把剛刪的檔案 resurrect。
  - 第二輪修正：`src/components/CommandPalette.tsx` 新增 `recentlyDeletedRef: useRef<Set<string>>`：(1) `refreshAfterFileMutation` 不再 re-fire search，只把 path 加入 ref + 從 `results` filter 掉；(2) chunk listener 用 ref 過濾 `payload.items`，攔截晚到的 stream chunk 帶回已刪 path；(3) `handleQueryChange` / `workspace-switched` / `workspace-cycled` 三個 listener 都 `clear()` ref，避免 cross-context 過濾錯。
  - 不影響 preview-only 路徑（confirm=false dry-run 不調 refresh）。
  - 檢查：`npx tsc --noEmit` 清、`npm run lint` 清；backend 無動。

- 2026-05-18: LAUNCH.2 Slice 1 (A + B) 交付：
  - **A — workspace-aware search (`handlers/search.rs`)**：新 pure helper `strip_global_prefix` (處理 `:global` / `:global foo` / 空白變體) + `SearchHandler::resolve_workspace_filter` (combines prefix strip 與 `workspace_manager.current().project_root`)。新 `apply_workspace_filter(&mut Vec<UiSearchItem>, Option<&str>)`：file/folder/app 限制在 root 內（case-insensitive prefix）；command/note/history/model 永遠保留。Sync (`execute_sync_query`) 與 stream (`execute_stream_query` + `run_stream_worker`) 兩條路徑都套用；`StreamWorkerRequest` 加 `workspace_root: Option<String>` 欄位。
  - **B — workspace cycle hotkey**：原 spec 訂 `Ctrl+Alt+W` 但已被 mouse cursor up 佔用，改 default `Ctrl+Alt+0`（與 workspace_1/2/3 同系列）。新設定 `hotkeys.workspace_cycle` (`default_config.toml` + `settings_schema.rs`)。`shortcuts.rs` 註冊：讀 config、計算 `(current+1) % SLOT_COUNT`、`switch_to(next)`、emit `workspace-cycled` event。前端 `CommandPalette.tsx` 加獨立 listener：清 `query` / `results` / `selected` / `cmdResult`，呼 `SEARCH_CANCEL`，refocus input。與原 `workspace-switched` (Ctrl+Alt+1/2/3) 不同點：cycle 強制清空 query，不還原 workspace 儲存的 query。
  - **檢查**：`cargo test` 344/345（+5 新測試：4 apply_workspace_filter cases + 1 strip_global_prefix variants；pre-existing nvim 不變）；`cargo clippy -- -D warnings` 清；`npm run lint` 清；`npx tsc --noEmit` 清。
  - **未做（下一批）**：LAUNCH.2.C per-workspace quick actions — 需 macro/hotkey override schema 與 UI。

- 2026-05-18: ONBOARD.1 Slice 1 (A + B + C) 交付：
  - **A — onboarding tour (`src/components/OnboardingTour.tsx` 新檔)**：4 步 modal (welcome / search / commands / customise)。localStorage `keynova.onboarding.completed` flag + `hasCompletedOnboarding()` / `markOnboardingCompleted()` / `resetOnboarding()` helper。Conditional mount in CommandPalette (`{onboardingOpen && <OnboardingTour … />}`) 保證每次 reopen 從 stepIdx=0 開始，避免 setState-in-effect。Esc/Skip → mark completed + close；Enter/→ next；← back。
  - **A backend — `OnboardCommand` (`src-tauri/src/handlers/builtin_cmd.rs`)**：`/onboard` builtin。前端 `execCommand` 攔截 name === "onboard"：`resetOnboarding()` + `setQuery("")` + `setOnboardingOpen(true)`，不送 backend result UI。後端 OnboardCommand fallback 回 Inline `"Replaying onboarding tour…"` 給任何讀 result 的 code path。
  - **B — `?` cheatsheet overlay (`src/components/CheatsheetOverlay.tsx` 新檔)**：query 為空時 `?` 觸發（避免與打 `?` 進 search 衝突）。4 個 section (Global / Search results / Secondary action menu / Onboarding tour)，列鍵位 + 描述。Esc / 再按 `?` / 點 backdrop 關閉。Conditional mount 同 A。v2 留：各 panel 自註冊鍵位（先用靜態清單足夠）。
  - **C — empty-state CTA (`src/components/CommandPalette.tsx`)**：search 模式 + `query.trim() !== ""` + `results.length === 0` + 非 pipeline 時，於 input bar 下方顯示 4 個 CTA chip：Create note "$query"（呼 `/note create $query`）、`/help`、`/setting`、Replay `/onboard`。
  - **註冊 (`src-tauri/src/app/state.rs`)**：`OnboardCommand` 加進 `build_builtin_registry`。
  - **檢查**：`cargo test` 339/340（pre-existing nvim test 不變）；`cargo clippy -- -D warnings` 清；`npm run lint` 清；`npx tsc --noEmit` 清。
  - **未做（下一批）**：ONBOARD.1.D Re-engage prompt 需 per-feature last-used timestamps 持久化；ONBOARD.1.E First-run hotkey 衝突偵測需跨平台 OS-level hotkey enumeration。

- 2026-05-18: UTIL.2.J killport 交付（UTIL.2 group 結案）：
  - **`src-tauri/src/core/process_lookup.rs` (新檔)**: `ProcessInfo { pid, process_name, port, protocol }`；`find_process_by_port` 跨平台 dispatch — Windows 走 `netstat -ano -p TCP` + `tasklist /FI "PID eq X" /FO CSV /NH` 二次查 process name；Unix 走 `lsof -nP -iTCP:PORT -sTCP:LISTEN`。`kill_pid` 跨平台：`taskkill /F /PID` 或 `kill -9`。Pure parsers (`parse_netstat_tcp_listen` / `parse_tasklist_csv` / `parse_lsof_listen`) 抽到 module 頂層給單元測試。
  - **`KillPortCmd` (`handlers/dev_utils_cmd.rs`)**: 兩段式 confirm — `killport <port>` 回 preview（pid + process_name + protocol + 操作提示），不殺；`killport <port> kill` 第二次 lookup 後實際 kill。第二次 lookup 是 TOCTOU 防護，避免 preview 後 process 已換成別的 pid。
  - **註冊**: `state.rs::build_builtin_registry` 加 `KillPortCmd`，dev utility 指令總計 15 個。
  - **安全邊界**: launcher-only（無 IPC route、無 agent tool）；與 LAUNCH.1.B 二段式 confirm 同 pattern；user-initiated → 不需 ADR。`kill_pid(0)` / `find_process_by_port(0)` guard。
  - **測試**: core::process_lookup 12 個（5 netstat parser + 3 tasklist + 2 lsof + 2 zero-guard）；handlers::dev_utils_cmd KillPortCmd 3 個（usage/invalid/unbound port）。完整 339/340（pre-existing nvim test 不變）；clippy `-D warnings` 清；docs:refresh 過。
  - **歸檔**: UTIL.2 group 整段搬至 `completed.md` (per docs/CLAUDE.md §5a)；backlog.md UTIL.2 段移除；Mainline History 加 row 12。

- 2026-05-18: UTIL.2 Dev Utilities Slice 1 (A–I) 交付（14 個 builtin commands）：
  - **新模組 (`src-tauri/src/core/dev_utils.rs`)**：pure-fn 計算層，無 manager actor。涵蓋 uuid v4 + nanoid (URL-safe alphabet)、`generate_password` (sym/alnum/alpha 三模式 + 二次 shuffle)、`hash_text` (md5/sha1/sha256/sha512)、`b64_encode/decode`、`url_encode/decode`、`json_pretty/minify`、`regex_test` (capture groups + 20 match cap)、`jwt_decode` (URL-safe base64 + serde_json pretty + `exp` 過期 hint)、`color_convert` (#hex/rgb()/hsl() 三制互轉，含短 hex `#0f0` → `#00FF00` 展開)、`cron_explain` (5/6/7 欄 auto-pad seconds + 下 5 次 fire local time)。
  - **BuiltinCommand wrappers (`src-tauri/src/handlers/dev_utils_cmd.rs`)**: 14 個 `BuiltinCommand` impls — UuidCmd、NanoidCmd、PwCmd、HashCmd、B64encCmd、B64decCmd、UrlencCmd、UrldecCmd、JsonCmd、JsonmCmd、RegexCmd、JwtCmd、ColorCmd、CronCmd。全部回 `CommandUiType::Inline`；invalid args 不 panic 而是回 `usage:` / `error:` 文字。
  - **註冊 (`src-tauri/src/app/state.rs`)**：14 個指令塞入 `build_builtin_registry`；皆無 state、無 manager 依賴，與既有 `CalCommand` 同一層。
  - **Cargo deps**：新增 `base64 = "0.22"`、`regex = "1"`、`md-5 = "0.10"`、`sha1 = "0.10"`、`cron = "0.12"`、`urlencoding = "2"`、`rand = "0.8"`。`uuid`、`sha2`、`chrono`、`serde_json` 已是既有依賴。
  - **測試**：core::dev_utils 22 個、handlers::dev_utils_cmd 9 個，總 31 個新增。`cargo test` 324/325（+31；pre-existing nvim test 不變）；`cargo clippy -- -D warnings` 清。
  - **未做（v2 / 下一批）**：clipboard fallback for hash/b64/json/etc.（前端工作）、regex replace preview、color 色塊預覽、UTIL.2.J killport（跨平台 process enumeration + 兩段式 confirm，工作量另算）。

- 2026-05-18: UTIL.1 Calculator++ offline parts 交付（A volume + B-offline + C date + D 既有）：
  - **UTIL.1.A volume**：`CalculatorManager::convert_unit` 擴 12 個 volume 單位（l/ml/cl/dl/m3/cup/tbsp/tsp/fl_oz/pt/qt/gal，US 標準），既有 length/weight/temperature/time/area/speed/data 不動。新增 5 個測試。
  - **UTIL.1.B-offline currency**：`try_currency_conversion` 新方法走 hard-coded USD-base 表（18 currencies: USD/EUR/GBP/JPY/CNY/TWD/KRW/HKD/SGD/AUD/CAD/CHF/INR/THB/MYR/PHP/IDR/VND）；結果末尾附 `(offline rate, snapshot 2026-05)` 提示 stale。Online 部分 blocked on ADR-038。新增 4 個測試。
  - **UTIL.1.C date arithmetic**：用 chrono `NaiveDate` + `Days`/`Months`/`Weekday` API；支援 `today`/`tomorrow`/`yesterday`、`today + N days/weeks/months/years`、`N <unit> ago`、`<date> - <date>` (days between)、`next/last <weekday>`；接受 `YYYY-MM-DD` 與 `YYYY/MM/DD`。CalculatorManager 加 `today_override: Option<NaiveDate>` 給測試固定參考日期。新增 8 個測試。
  - **UTIL.1.D base conversion**：既有功能 (`0xFF`、`10 to bin`、`255 to hex`、`0o17`)，標記完成。
  - **ADR-038**：`docs/adr/0038-currency-online-rates.md` 草擬完成（status: 提議）；涵蓋 exchangerate.host / open.er-api 雙 provider、24h 快取、TLS 強制、2s/5s timeout、無使用者資料外流、`calculator.currency_online_enabled` 預設 false。`docs/decisions.md` + `docs/tasks/blocked.md` 同步登錄。
  - **檢查**：`cargo test` 293/294（+17：5 volume + 4 currency + 8 date；pre-existing nvim test 不變）；`cargo clippy -- -D warnings` 清。
  - **未動作**：UTIL.1 group 因 `UTIL.1.B-online` 待 ADR-038 接受，**不搬移 completed.md**（per docs/CLAUDE.md §5a，group 需 100% 才能整段歸檔）。

- 2026-05-18: LAUNCH.1.C/D/E 全段交付（LAUNCH.1 group 結案）：
  - **Slice 1（後端基礎）**：`UiSearchItem` 加 `score_breakdown { base, recency_boost, frequency_boost }`（`#[serde(default)]`）；`SearchManager::rank_boost_breakdown(source, path) -> (i64, i64)` 取代舊 `rank_boost`；`apply_rank_boost` 寫入三段拆解，總分仍為 base+boost 維持排序穩定。新檔 `src-tauri/src/core/preview.rs` 提供 `classify_path` / `read_text_preview` / `guess_image_mime` shared helpers，`LearningMaterialManager::preview_file` 改呼叫之。`handlers/file.rs` 加 `preview` arm（typed `FilePreviewRequest`，text 4 KB / 500 行 default、64 KiB / 2000 行 cap，image 只回 metadata + mime，binary 只回 metadata）。`tauri.conf.json` 開啟 `assetProtocol { enable: true, scope: ["**"] }`，`Cargo.toml` 加 `protocol-asset` feature。`[search]` section 加 `preview_enabled` / `show_rank_breakdown`。測試：file 6 個 + core::preview 7 個 + search_manager 4 個 = 17 個新增。
  - **Slice 2（filter chips）**：新 `src/components/FilterChips.tsx`（6 個 chip、KIND_BADGE 對齊色系、`localStorage["keynova.searchFilters"]` 持久化）。`src/types/search.ts` 加 `SourceFilter` / `ScoreBreakdown` / `FilePreviewResult`。`CommandPalette` 衍生 `visibleResults`（`folder` 對應 `file` chip）、`safeSelected` render-time clamp 取代 setState-in-effect；raw > 0 但 visible == 0 時顯示「Clear filter」escape hatch。
  - **Slice 3（preview pane + rank tooltip + 動態寬）**：新 `useFilePreview.ts`（80 ms debounce、LRU 64 cache、`cancelled` flag）、`PreviewPane.tsx`（text/image/binary 三分支、`convertFileSrc` 走 asset protocol）、`RankTooltip.tsx`（fixed pos、自動翻邊）。`useWindowResize.ts` 加 `widthRef` 參數 + `PALETTE_WIDTH_NARROW`(640) / `PALETTE_WIDTH_WIDE`(960)。`CommandPalette` 用 `showPreview` 控 `paletteWidthRef` 切換並觸發 resize；result container grid-cols-[1fr_320px]；`relative` anchor 搬到 inner left wrapper（SecondaryActionMenu 不漂進 preview 欄）；`hover: {index, rect}` 單一 state 取代 ref-during-render。
  - **檢查**：`cargo test` 276/277（pre-existing nvim test 不變）；`cargo clippy -- -D warnings` 清；`npx tsc --noEmit` 清；`npm run lint` 清。
  - **文件**：`docs/security.md` 新增 §10 `Tauri Asset Protocol` 邊界說明。LAUNCH.1 group 全段搬至 `completed.md`，`backlog.md` 移除整段。

- 2026-05-18: LAUNCH.1.A 收尾 + LAUNCH.1.B 全段交付：
  - **後端 (`src-tauri/src/handlers/file.rs`)**: 在既有 `reveal` / `open_with` 之上新增 5 個 match arm — `rename` / `move` / `delete` / `hash` / `open_as_text`。Destructive 三個（rename/move/delete）走二段式 confirm gate：`confirm != true` 回 `{ preview: true, ... }`，`confirm: true` 才真執行。`delete` 用 `trash::delete()`（OS recycle bin），`hash` 串流 64 KiB chunks 餵 `sha2::Sha256`，`open_as_text` 用 `text_editor_for_platform()` helper（Windows `notepad.exe` / macOS `TextEdit` / Linux fallback）。
  - **Typed DTO (`src-tauri/src/models/ipc_requests.rs`)**: 新增 `FileRenameRequest` / `FileMoveRequest` / `FileDeleteRequest` / `FileHashRequest` / `FileOpenAsTextRequest`；handler arm 全走 `serde_json::from_value::<DTO>`。
  - **Cargo deps (`src-tauri/Cargo.toml`)**: `trash = "5"`、`sha2 = "0.10"`、`tempfile = "3"` (dev-dep)。`trash` 提供跨平台 OS recycle bin（Windows SHFileOperationW / macOS NSFileManager trashItem / Linux freedesktop gio），MIT/Apache-2.0；停留在既有 file boundary 內，不需 ADR。
  - **前端 (`src/utils/secondaryActions.ts`)**: 解除 `open_with` 的 `disabled`；依 `result.kind` 條件性追加 `open_as_text` (file only) / `rename` / `move` / `delete` (non-app) / `hash` (file only)。新增 `isDestructive()` 與 `parentDirFromPath()` helper。
  - **前端 (`src/components/SecondaryActionMenu.tsx`)**: 加 `pendingConfirmId` / `inlineInput` / `onInlineInputChange` / `onInlineInputKeyDown` props；focused destructive 項目下方渲染紅框 confirm row；rename/move 焦點時於該列下方渲染 inline `<input>`（autoFocus）。動態 risk 色彩（high→red、medium→amber、low→gray）。
  - **前端 (`src/components/CommandPalette.tsx`)**: 新 state `pendingConfirm` / `inlineInput`；新 helper `showHint(msg, durationMs)`。`handleSecondaryAction` 6 個 case 全填：`open_with` / `open_as_text` 直接 dispatch 並提示；`hash` dispatch 後自動複製 hex 到 clipboard；`rename` / `move` 先開 inline input，Enter → dry-run preview → 再 Enter 才真執行；`delete` 第一次 Enter → dry-run preview（size + destination） → 再 Enter 才真送 trash。Esc 全域與選擇切換都清空 `pendingConfirm` + `inlineInput`。
  - **測試 (`src-tauri/src/handlers/file.rs::tests`)**: 16 個（11 新 + 5 既有），含 `rename_preview_when_confirm_false`、`rename_executes_when_confirm_true`、`rename_rejects_existing_target`、`rename_rejects_path_separator_in_new_name`、`move_preview_returns_source_and_target`、`move_rejects_non_directory_target`、`delete_preview_reports_destination_recycle_bin`、`delete_moves_to_trash_when_confirmed`（`#[ignore]` — 需 desktop session）、`hash_sha256_matches_known_vector`（`b"abc"` → `ba7816bf…`）、`hash_rejects_unknown_algorithm`、`open_as_text_requires_path`。
  - **檢查**：`cargo test handlers::file` 16/16 (1 ignored)；`cargo test` full suite 258/259 (`note_lazyvim_missing_nvim_returns_inline_guidance` pre-existing 不變)；`cargo clippy -- -D warnings` 清；`npx tsc --noEmit` 清；`npm run lint` 清。
  - LAUNCH.1.C/D/E 仍留在 backlog。LAUNCH.1.A / LAUNCH.1.B 已搬至 `completed.md`。

- 2026-05-17: LAUNCH.1.A slice 1 scaffold in progress (Phase 8b kickoff):
  - Frontend: `SecondaryActionMenu` component + `secondaryActions` util + CommandPalette keyboard wiring (open/close, focus index, metadata expand, copy hint) + `@tauri-apps/plugin-opener` reveal integration.
  - IPC: reserved 8 `file.*` route constants (`open_with` / `reveal` / `rename` / `move` / `delete` / `hash` / `preview` / `open_as_text`) for slice 2-4 backend wiring.
  - Backend: `FileHandler` skeleton registered in `build_command_router`; concrete dispatch arms pending.
  - Out-of-scope sidecar in same commit: `Prompt_Engineering_Init_Template.docx` + `scripts/generate_prompt_engineering_doc.py` — cross-project reusable prompt-engineering bootstrap derived from current CLAUDE.md + docs/ pattern.

- 2026-05-16: Phase 7a Agent / AI panel UX upgrade complete — AGENT.1.A–E + AGENT.2.A–C + AGENT.7.A/B/C/D (12 sub-items, 9 slices):
  - **Slice 1**: `src/ipc/routes.ts` — AI/Agent route constants (`AI_CHAT`, `AI_CANCEL`, `AGENT_START`, …); `useAi.ts` + `useAgent.ts` migrated off literal strings; `AiHandler.cancel_flags: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>` scaffolded.
  - **Slice 2 (AGENT.1.C 後端)**: `ai.cancel` IPC route; `chat_async(cancel_flag, cancel_registry)` short-circuits before/after `do_chat`, rolls back the user history entry on cancel, emits `ai.response { cancelled:true }`, and self-cleans the registry on completion.
  - **Slice 3 (AGENT.1.A 後端)**: SSE/NDJSON streaming for Ollama / OpenAI / Claude; each chunk emits `ai.stream.chunk`; new `ai.stream_enabled` setting (default `true`); pure parsers (`parse_ollama_chunk`/`parse_openai_chunk`/`parse_claude_chunk`) + `accumulate_stream` accumulator unit-tested.
  - **Slice 4 (AGENT.1.A/C/E 前端)**: `useAi.ts` listens for `ai-stream-chunk`, exposes `cancel()`; new `<ElapsedTimer>` ticks every 500 ms and surfaces a `Long running — Cancel?` hint at 30 s; Agent panel Cancel button now also shows while `running` / `planning`.
  - **Slice 5 (AGENT.1.B/D)**: `react-markdown` + `remark-gfm` + `rehype-highlight` added; new `<Markdown>` wrapper (raw HTML disabled, `<pre>` ErrorBoundary fallback); `src/utils/aiErrors.ts` classifier covers `connection_refused`/`model_not_found`/`unauthorized`/`timeout`/`oom`/`cancelled`; `<ErrorCard>` exposes Open-AI-settings / external-doc / Retry CTAs. Also fixed FEAT.11 typo: `useIPC` → `useIPCContext` in `LearningMaterialPanel.tsx`.
  - **Slice 6 (AGENT.2.A/B)**: Hover-revealed Copy / Regenerate per assistant + Agent final-answer card; `useTextareaAutosize` (rows 1–8); `useLocalHistory(storageKey, cap)` backs ↑/↓ recall against `keynova.chatHistory` / `keynova.agentHistory` (cap 50 each, dedupe).
  - **Slice 7 (AGENT.2.C)**: New `agent.show_audit_by_default` setting (default `false`); audit `<details>` honours it on mount.
  - **Slice 8 (AGENT.7.C)**: `AgentRuntime` now owns `RunStore { runs, order }` and an `AgentArchiveSink` trait; `insert_run` enforces FIFO cap (default `agent.run_history_cap = 20`), archives evicted runs through `KnowledgeStoreArchiveSink → agent_archive` table (new SQLite migration), and publishes `agent.run.archived`; frontend `useAgent.archivedCount` + AiPanel hint surface the eviction.
  - **Slice 9 (AGENT.7.A/B/D)**: `AgentApproval` gains `tool_name` / `deadline_unix_ms` / `remember_for_run` (`#[serde(default)]`); approval timeout configurable via `agent.approval_timeout_secs` (default 300); `wait_for_react_approval` now mutates the approval to `"approval_timeout"` and emits `agent.approval.timeout`; `agent.approve` accepts `remember: bool`; ReAct loop short-circuits the gate when an earlier approval for the same `tool_name` had `remember_for_run=true`. New `<ApprovalCard>` consolidates `<CountdownPill>` (60s amber / 10s red / "Timed out"), the **Approve and remember for this run** checkbox, an `<ApprovalSummary>` dispatcher across 8 `AgentActionKind`s, and a `Show raw` fallback.
  - Tests: 242/243 backend (1 pre-existing failure unchanged); new Slice-2 cancel tests, Slice-3 streaming parser + accumulator tests, Slice-8 FIFO eviction test, Slice-9 approval-timeout test.
  - Settings additions: `ai.stream_enabled`, `agent.show_audit_by_default`, `agent.run_history_cap`, `agent.approval_timeout_secs`.
  - Lint / tsc / clippy `-D warnings` all green.

- 2026-05-15: FEAT.11 Learning Material Review complete (FEAT.11.A–J):
  - `docs/adr/0028-learning-material-review-local-context.md`: ADR created (提議 status); workspace-root scope, symlink escape prevention, denylist, disabled-by-default.
  - `models/learning_material.rs`: `MaterialClass`, `MaterialCandidate`, `ScanStats`, `ReviewReport` with `to_markdown()`.
  - `managers/learning_material_manager.rs`: metadata-first scanner, `classify_by_extension`, `is_project_root`, `is_denied` (glob + exact), `preview_file` with secret redaction; DEFAULT_DENYLIST covers .env/.key/.pem/id_rsa/etc.
  - `handlers/learning_material.rs`: `scan`, `preview`, `export_note`, `export_markdown` IPC commands; canonicalized path write for markdown export.
  - `core/agent_runtime.rs`: `LearningMaterialReviewToolParams/Result` (JsonSchema); `learning_material.review` agent tool registered (ApprovalPolicy::Required, ActionRisk::Medium).
  - `handlers/agent/mod.rs`: `dispatch_learning_material_review()` wired into approval-gated dispatch.
  - 5 settings under `agent.local_context` (disabled by default); `default_config.toml` updated.
  - `src/components/LearningMaterialPanel.tsx`: full scan UI with stats bar, class filter tabs, candidate list, export-as-note action.
  - `src/ipc/routes.ts`: 3 new IPC route constants.
  - 13/13 FEAT.11 tests pass; 233/234 total (1 pre-existing failure unchanged); clippy clean.

- 2026-05-15: P3 Context Compiler Lite complete (P3.A + P3.B):
  - `models/context_bundle.rs`: `ContextBundle` struct with `WorkspaceContext`, `SelectedFileContext`, `ContextTokenBudget`; `build()` with 2-pass token budget.
  - `AgentHandler::build_context_bundle()`: manager-only assembly; no FS scan; bounded snippet preview (200 chars); 3000-char total cap.
  - `AgentRun.context_bundle: Option<ContextBundle>` (`#[serde(default)]`); both run paths wired.
  - Frontend types aligned in `useAgent.ts`.
  - 220/221 tests pass (1 pre-existing failure unchanged).
- 2026-05-15: AI panel UX hotfix delivered for clear behavior and chronology:
  - `AGENT` mode now has backend clear command (`agent.clear_runs`) and frontend clear wiring.
  - Agent runs are rendered in chronological order (old -> new) to keep the newest request at the bottom.
  - Chat history hydration now avoids stale overwrite after local send/clear actions.
- 2026-05-15: Setting panel tab bar layout adjusted for dense section lists:
  - replaced equal-width tab squeeze (`flex-1`) with content-width tabs.
  - enabled horizontal scrolling for section tabs to avoid overlap/truncation collisions.
  - added fallback title-casing for unmapped dynamic section labels.
- 2026-05-15: Setting panel tab visual polish pass:
  - tab rail now uses subtle dark gradients on both edges to match panel tone.
  - horizontal scrollbar themed to dark/slate and slimmed down to reduce visual noise.
  - tab typography tuned to surrounding UI scale (12px semibold, tighter spacing).

See `docs/tasks/backlog.md` for detailed breakdown.
See `docs/tasks/blocked.md` for gating constraints.
See `docs/tasks/completed.md` for P0/P1/P2 history.
