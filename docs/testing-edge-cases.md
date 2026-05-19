---
type: testing_policy
status: active
priority: p1
updated: 2026-05-19
context_policy: retrieve_when_debugging
owner: project
---

# Keynova 邊緣案例測試矩陣（模擬真實使用者）

> Retrieval policy: 排查 launcher bug、寫新 regression test、預先攔截使用者情境時讀取。本檔不取代 `docs/testing.md` 的核心策略；專注列舉「會踩到實際使用者環境特殊性」的測資。

**動機**：2026-05-19 Bug B（→ → Enter 後 row 消失但桌面檔案仍在）在標準 tempdir / 一般 OneDrive 檔案 / 中文檔名 / `.lnk` 捷徑 / 資料夾的後端測試全部正確 trash。

> **2026-05-19 更新（假說否決）**：原本懷疑「藍雲 OneDrive Files On-Demand placeholder 被 trash 後雲端 re-sync 回原 path」。實測：強制 `Free up space` 把測試檔 dehydrate 為 placeholder（attrs `0x100020` = ARCHIVE + RECALL_ON_DATA_ACCESS），呼叫 `FileHandler.execute("delete", confirm=true)`，**T+0s 到 T+120s 持續 Test-Path=False**，placeholder 沒被 sync 回來。檔案以完整 2030 bytes 進入 Recycle Bin（OneDrive trash 自動 hydrate 完整內容並刪雲端複本）。**Bug B 真根因不在 OneDrive Files On-Demand**。

這份文件補足之前測試矩陣對「使用者真實機器狀態」的覆蓋盲區。

> **2026-05-19 Bug B 真根因（最終結論）**：使用者實測加 trace 後確認：(B1) **launcher 已正確 trash + verify**，「桌面內容沒刪除」是 **Windows Explorer desktop view 不主動 refresh** — `SHFileOperationW` 發 `SHCNE_DELETE` 但 OneDrive filter driver / 多 Explorer instance / Defender backlog 會吞掉或延遲通知。按 F5 即更新。(B2) **使用者從 Recycle Bin 還原檔案後，launcher 搜尋找不到** — 因為 `CommandPalette.tsx::recentlyDeleted` kill-set 在同一 query 下 sticky 過久。修法：B1 在 success hint 加「F5 on desktop if icon lingers」誠實揭露 OS 行為；B2 把 kill-set 從 `Set<string>` 改 `Map<string, number>` + 30s TTL，restore 後 30s 內 search 仍藏、之後自然回來。兩個 fix 都純 frontend、無 backend 動。

---

## 1. OneDrive sync 狀態矩陣（最高優先）

| OneDrive icon | 屬性位元 | 物理狀態 | trash 預期 | 已測 |
|---|---|---|---|---|
| 綠 ✓（永遠保留）| `ARCHIVE \| PINNED (0x80020)` | 完整本機 + 雲端 | 真 trash + 雲端 sync 也 delete | — |
| 綠 ✓（已下載）| `ARCHIVE (0x20)` | 完整本機 + 雲端 | 真 trash + 雲端 sync 也 delete | ✓（已測過普通 .txt） |
| **藍 ☁（雲端唯一）**| `ARCHIVE \| RECALL_ON_DATA_ACCESS (0x100020)` | 僅本機 reparse point + 雲端 | 完整內容 hydrate 進 Recycle Bin + 雲端複本刪除 | **✓ 已測（120s 內不會 resync 回來）** |
| 雙圓 ↻（同步中）| 變動中 | 部分傳輸中 | 不可預測：trash 可能與 sync 競態 | — |
| 紅 ⊗（同步錯誤）| 變動 | 可能不一致 | trash 可能成功但 sync 卡住 | — |

### Repro 步驟（藍雲 / Files On-Demand placeholder）

```powershell
# 1. 在 OneDrive Desktop 建檔，存內容
$desktop = [Environment]::GetFolderPath('Desktop')
$path = Join-Path $desktop "fod-test.txt"
Set-Content -Path $path -Value "to be cloud-only" -Encoding utf8

# 2. 等 OneDrive 上傳（檢查任務列圖示是否轉為 green check）
Start-Sleep -Seconds 30

# 3. 右鍵 → "Free up space"（或 attrib +U $path 然後等 OneDrive 處理）
attrib +U $path

# 4. 等 OneDrive client 把 local content reclaim、icon 變藍雲（可能要 10-60s）
Start-Sleep -Seconds 60
Get-Item $path | Select-Object Attributes
# 預期含 ReparsePoint + Offline，或 RecallOnDataAccess

# 5. 跑 keynova repro_delete_path_from_env
$env:KEYNOVA_REPRO_DELETE_PATH = $path
cargo test --manifest-path src-tauri/Cargo.toml repro_delete_path_from_env -- --ignored --nocapture

# 6. 立即檢查 Path::exists（應 false）
# 7. 等 30~120s 後再檢查 Path::exists / Get-Item $path（觀察是否 re-sync 回來）
```

**期待找出**：
- trash::delete 對 reparse point 的回傳碼
- `verify_path_removed` 在 trash 後立即回 Ok（檔案 NotFound）
- 60s 後 OneDrive 是否補回 placeholder

**修法候選（不在本文範圍）**：
- A. delete 前偵測 `IO_REPARSE_TAG_CLOUD_*`（Windows Cloud Filter API），refuse 並提示「請先 right-click → Always keep on this device」
- B. delete 前若是 placeholder，先 trigger download（讀檔 1 byte），等變為實體檔再 trash
- C. 偵測後直接呼叫 OneDrive 的 cloud delete API（複雜，需 MSAL 認證）

---

## 2. 檔案類型邊緣案例

| 類型 | 路徑範例 | 已測 | 風險 |
|---|---|---|---|
| 普通檔案 | `C:\...\foo.txt` | ✓ | low |
| 資料夾 (recursive) | `C:\...\folder\` | ✓ | medium（深度大時 trash 慢） |
| Windows `.lnk` 捷徑 | `Desktop\App.lnk` | ✓（delete `.lnk` 本身） | low |
| **NTFS Symlink** | `mklink link target` 後的 link | — | trash 是刪 link 還是 target？需驗 |
| **NTFS Junction** | `mklink /J junction targetdir` | — | 同上 |
| **Hard Link** | `mklink /H hardlink target` | — | trash 應只刪 hardlink 名 |
| **Sparse File** | `fsutil sparse setflag` 後的 file | — | trash 對 sparse 行為 |
| **OneDrive placeholder** | 藍雲檔 | **— 未測** | **高（Bug B 候選根因）** |
| **iCloud / Google Drive placeholder** | iCloud Drive sync 檔 | — | 類似 OneDrive 風險 |
| 唯讀檔 | `attrib +R foo.txt` | — | trash 可能 prompt 確認或失敗 |
| 系統檔 | `attrib +S foo.txt` | — | trash 通常 refuse |
| 隱藏檔 | `attrib +H foo.txt` | — | 搜尋是否會顯示？ |
| 加密檔 (EFS) | `cipher /E foo.txt` | — | trash 行為 |
| 大檔（>2GB） | 2GB+ video | — | trash 時間 + verify timeout |
| 0-byte 檔 | `New-Item -ItemType File` | — | preview/hash 邊界 |

---

## 3. 路徑邊緣案例

| 情境 | 路徑範例 | 已測 | 風險 |
|---|---|---|---|
| ASCII | `C:\foo\bar.txt` | ✓ | — |
| 中文 | `C:\專案\測試.txt` | ✓ | — |
| 日文 / 韓文 | `C:\テスト\파일.txt` | — | 編碼往返 |
| Emoji 😀 | `C:\😀\file.txt` | — | UTF-16 surrogate pair |
| 空格 + 特殊字元 | `C:\my docs\file (1) & test.txt` | — | shell escape |
| 保留名 | `C:\con.txt` `nul.txt` `prn.txt` | — | Windows 拒絕建立 |
| Long path (>260 chars) | 深層巢狀 | — | 需 `\\?\` prefix 才能 access |
| UNC path | `\\server\share\file.txt` | — | trash 對網路檔 |
| 映射磁碟 | `Z:\file.txt`（map 到 `\\server\share`）| — | 同上 |
| USB removable | `E:\file.txt` | — | Recycle Bin 可能 disabled |
| WSL path | `\\wsl$\Ubuntu\home\user\file` | — | ext4 vs NTFS |
| 8.3 短名 | `C:\PROGRA~1\foo` | — | trash 是否處理 |
| 大小寫變異 | `C:\Foo.TXT` vs index 內 `c:\foo.txt` | — | Windows 大小寫不敏感 |
| 正反斜線混用 | `C:/foo\bar.txt` | — | 標準化問題 |
| Trailing whitespace | `"C:\foo.txt "` | — | trim_path 已處理 |

---

## 4. 檔案 lock / 權限狀態

| 狀態 | Repro | 風險 |
|---|---|---|
| **檔案在 Notepad 打開** | 開啟 + 鎖定後 launcher delete | trash 可能 silent ok 但實際失敗 — Bug B 候選 |
| **檔案在 Word/Excel 打開（FILE_SHARE_NONE）**| 同上 | 同上但更嚴格 |
| **檔案在防毒軟體掃描中** | 大檔案剛複製 | trash 暫時被擋 |
| **檔案剛建立，OS 尚未 flush** | `Set-Content` 後立即 delete | metadata 競態 |
| **No-write ACL** | 用 icacls 移除 write 權 | trash 失敗，需明確 hint |
| **OwnedBySystem** | 系統建立的檔 | trash 可能要 admin |
| **多人協作中（OneDrive co-edit）** | 雲端他人正在編輯 | sync 衝突 |

---

## 5. UI 鍵盤時序 / IME 狀態

| Scenario | Repro 步驟 | 期待 | 風險 |
|---|---|---|---|
| 單按 Enter | `→ → Enter` | Preview hint + row 保留 | low |
| 雙按 Enter（慢） | `→ → Enter`, 等 1s, `Enter` | 第二次 Enter trash | low |
| 雙按 Enter（快） | `→ → Enter Enter`（<200ms） | **可能 race**：兩次 confirm=false | medium |
| 按住 Enter | `→ →`, 按住 Enter 1s | OS repeat 觸發多次 keydown，可能 confirm=false→true→Err（檔已沒）→preview... | high — 潛在 Bug B 變體 |
| Enter + Esc 競態 | `Enter, 立即 Esc` | Esc 清 pendingConfirm，row 保留 | low |
| Enter 後改 query | `Enter`(preview), 改 query, `Enter` | 新 result 被 trash？ | high — `result` closure 是否 stale |
| **IME composition 中按 Enter** | 中文輸入未確認字 → Enter 應送 IME 確認，不該觸發 launcher Enter | composition end → 不送 trash | medium |
| **IME composition 中按 →** | 同上 | 不該開 secondary menu | medium |
| 雙語輸入切換中 | Ctrl+Space / Win+Space 切 IME 時 | window focus blip | Bug A 範圍 |
| 全域 hotkey 衝突 | 其他 app 也綁 Ctrl+K | Tauri 是否搶得到 | low |

---

## 6. Window / Focus 狀態（Bug A 範圍）

| Scenario | 期待 | 已測 |
|---|---|---|
| 純英文連續打字 30s | 不收起 | round-2 修法後預期 ✓ |
| 中文 IME 連續打字 30s | 不收起 | round-2 修法後預期 ✓ |
| 切到別 app 1.5s+ 後 | 自動收起 | round-2 修法後預期 ✓ |
| Win+Tab 切視窗瞬間 | 不收起 | — |
| Alt+Tab 短按 | 不收起 | — |
| 螢幕鎖定 / 解鎖 | 不收起 | — |
| 多螢幕、滑鼠移到別螢幕 | 不收起（window 不變焦） | — |
| 開 DevTools (F12) | DevTools 開時不收起 | — |
| RDP / 遠端桌面連接 | focus 巨變 | — |
| 喚醒後 (sleep/resume) | 應正常 | — |
| Composition 跨 onCompositionEnd 後 ↑↓ navigate | 不收起 | — |

---

## 7. 搜尋 backend / index 狀態

| Scenario | Repro | 風險 |
|---|---|---|
| Everything index stale | 外部 delete 後 launcher 搜尋仍找到 | `filter_existing_file_results` 應擋掉 |
| Tantivy index stale | 重新索引前 | 同上 |
| APP_CACHE stale | 同上 | 同上 |
| Recycle Bin 入索引（Everything 預設掃 `$Recycle.Bin`） | trash 後 search 又找到 | 已有 `recentlyDeleted` 攔截 |
| 多 backend 並發回 stale + fresh | 顯示順序錯亂 | — |
| 搜尋字串含 `:global` prefix | `apply_workspace_filter` skip | ✓ 有測 |
| 搜尋字串含 ` ` (前後空白) | trim 行為 | — |
| 空字串 / 純空白 | 應不發 IPC | — |
| 極長字串（10k 字元） | backend timeout / OOM | — |
| 正則 metachars（`.*?[]()` 等） | 不該觸發 regex 解釋 | — |

---

## 8. Workspace / 多 instance 狀態

| Scenario | Risk |
|---|---|
| Workspace cycle (Ctrl+Alt+0) 中 → delete | result list 已清 vs 新增 race | 
| Workspace 切換時 stale `recentlyDeleted` 是否清 | active.md 提過已清 |
| Project root 變動但 search 結果仍是舊 root | filter 應重做 |
| 多個 keynova process（不該發生但 robustness 上）| 鎖檔衝突 |

---

## 9. SecondaryActionMenu / pendingConfirm 狀態

| Scenario | Risk |
|---|---|
| pendingConfirm=delete 時切換 selection（↓↑）| 應清 pendingConfirm（active.md ESC 也清）|
| pendingConfirm=delete 時用滑鼠點別 row | 同上 |
| pendingConfirm=rename 開 inlineInput 後 → ← | inlineInput 是否被吃掉 |
| inlineInput 含路徑分隔字元（`\`）| backend 已測拒絕，frontend 該 hint |
| inlineInput 為空 | rename 為空名 → fs::rename 行為 |
| inlineInput target 已存在 | rename 失敗 hint |
| Hash 大檔（>1GB）時按 Esc | 是否能 cancel 串流？目前無 cancel token |
| Open with 對 .exe 檔 | shell execute 安全邊界 |

---

## 10. 跨功能組合（onboarding / cheatsheet / pipeline）

| Scenario | Risk |
|---|---|
| Onboarding tour 開著 → 按 Ctrl+K → 應仍 dismissable | useEffect listener stacking |
| `?` 開 cheatsheet 同時收到 search chunk | UI 重疊 |
| Pipeline (`a \| b`) 執行中按 → | secondary menu 不該開 |
| `/onboard` 在搜尋有結果時觸發 | 應跳到 onboarding，清 query |
| Empty-state CTA chip 「Create note」內含 query 特殊字元 | escape |

---

## 11. 多語言 / 環境變數

| Scenario | Risk |
|---|---|
| 系統語言為日文 / 韓文 / 阿拉伯文 | UI 文案是否 hardcode 中文 / 英文 |
| Locale 影響數字格式（千分位 `,` vs `.`）| size 顯示 |
| 時區變動 | `cron_explain` next 5 fire time 顯示 |
| 系統時間錯（年份在過去）| recency boost 計算 |

---

## 12. 自動化新增的測試（建議優先順序）

優先序 P0（直接關聯目前 bug 風險）：
1. **OneDrive placeholder delete repro**（§1 步驟）— 不可自動化，但寫成 cargo test 文件
2. **檔案 lock + delete repro** — 用 `tempfile` + `File::open_share_none`，再呼叫 FileHandler.execute(delete)，期待 Err
3. **Held Enter 模擬** — frontend Jest/Vitest 測 onKeyDown 重複觸發時 pendingConfirm 機制

優先序 P1：
4. Symlink / Junction / Hardlink trash 行為
5. UNC path / 映射磁碟 trash
6. Long path (>260) handling
7. 唯讀檔 / 系統檔 trash

優先序 P2：
8. Emoji / 多 surrogate 檔名
9. 跨時區 cron / recency
10. Pipeline + secondary menu 互動

---

## 13. 已知不會解決的（platform 限制）

- ~~**OneDrive Cloud-only re-sync race**：trash 那一瞬間 verify pass，但事後 OneDrive 補回，純 client-side 無法 100% 偵測。修法只能是「delete 前先 hydrate placeholder 成實體檔」。~~ **2026-05-19 已測否決**：placeholder trash 後 120s 內不會 resync 回來，OneDrive 同步 cycle 把完整內容下載進 Recycle Bin 並刪雲端複本。Bug B 真根因不在這。
- **Recycle Bin 在 D:/E: removable 上未啟用時**：trash 預設靜默 fallback 為永久刪除（trash crate v5 行為），無法防範。應加 hint。
- **跨 user / admin 邊界**：launcher 以一般 user 跑，遇 SYSTEM-owned 檔只能 refuse。

---

## 14. 報告格式建議

未來 bug repro 報告請依此格式提供，便於定位：

```
- Build commit: <git rev-parse HEAD>
- OS / 版本: Windows 11 26200 (build)
- OneDrive 狀態: green/blue/syncing/error
- 目標檔案路徑: C:\Users\...\file.txt
- 檔案 attribute: <Get-Item $path | Select Attributes>
- 操作步驟: 1. ... 2. ... 3. ...
- 預期: ...
- 實際: ...
- IPC reply（DevTools console）: ...
- Backend log（如有）: ...
```

### 14.1 file.delete trace log（2026-05-19 加入）

`src-tauri/src/handlers/file.rs::trace_delete` 在 debug build 中對 stderr 輸出每一個 stage：

```
[keynova][file.delete] stage=request      path="..." confirm=true raw_path="..."
[keynova][file.delete] stage=metadata_ok  path="..." kind=file size=Some(N) attrs=0xHEX[FLAG|FLAG] symlink=ok(file=... dir=... symlink=...)
[keynova][file.delete] stage=trash_invoke path="..." calling trash::delete
[keynova][file.delete] stage=trash_ok     path="..." trash::delete returned Ok
[keynova][file.delete] stage=verify_ok    path="..." post-trash symlink_metadata=NotFound
```

失敗路徑：
- `stage=metadata_err` → 路徑不存在 / 無權限讀 metadata
- `stage=trash_err` → trash crate 回 Err（明顯失敗）
- `stage=verify_err` → trash 回 Ok 但檔案還在（silent-failure 已被攔截）

**Bug B repro 時請依此步驟**：
1. 打開 terminal 跑 `npm run tauri dev`（stderr 才會顯示）
2. 重現問題 → 完整 trace 自動印在 terminal
3. 把 `[keynova][file.delete] stage=...` 那 5 行貼回給維護者

`stage=verify_ok` + 桌面檔案仍在 = backend 確實 trash 了，但別處有東西把檔案還原。可能性：sync 軟體 / 防毒復原 / shell view lag。
`stage=trash_ok` 後沒有 `verify_ok` = trash silent-failure（罕見）。
`stage=trash_err` = trash 直接失敗（最常見，明確錯誤路徑）。

Release build：trace 為 no-op（`#[cfg(debug_assertions)]` 守護），無 runtime 成本。
