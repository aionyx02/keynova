# 改用原生 Win32 取得 shell 圖示，停止對 PowerShell 的執行期依賴

**狀態：** 提議
**日期：** 2026-06-13
**決策者：** 開發者 / AI agent
**相關文件：**

- docs/architecture.md
- docs/memory/sessions/2026-06-13.md

---

## 1. Context（技術背景）

`platform/windows/icon.rs` 目前以 `powershell.exe` 執行內嵌腳本，呼叫 `SHGetFileInfo`
取得 HICON、用 `System.Drawing`（GDI+）轉成 PNG、再回傳 base64。

- 每一個 icon = 一個 powershell 子行程（`run_icon_script` → `.output()`）。
- 啟動 pre-warm（`warm_icon_cache`）一次噴 50–80 個 powershell（folder + 31 副檔名 + 每個 app）。
- 執行期 cache-miss（首次開面板、裝新 app）每列再 spawn 一個 powershell，約 150–300ms。

問題面向：UI responsiveness（icon 慢慢浮出）、CPU（開機尖峰）、以及對 `powershell.exe`
存在/可執行的隱性執行期依賴。剛完成的 `CREATE_NO_WINDOW` 修正已消除黑窗閃爍與焦點竊取
（見 `sessions/2026-06-13.md`），但 spawn 成本本身仍在。

## 2. Constraints

- 平台：Windows 10+（此路徑僅 Windows）。
- 輸出契約不變：仍回傳 `data:image/png;base64,...`，前端與磁碟快取（`.b64`）格式不動。
- 冷啟動 < 200ms（UI 可見）；記憶體目標冷啟 < 120MB。
- 不得回退到會跳視窗的子行程方案。

## 3. Alternatives Considered

### 方案 A：原生 Win32 + `png` crate（選定）

`SHGetFileInfoW` → HICON → `GetIconInfo` → `GetDIBits` 取 32bpp 像素，BGRA→RGBA（必要時用
AND mask 補 alpha），以 `png` crate 編碼，再 base64。

- 優點：完全不開子行程；icon 即時、開機無尖峰；移除 powershell 執行期依賴。FFI 比 GDI+ 單純，
  無 GDI+ 全域 init/shutdown 生命週期。
- 缺點：新增一個輕量外部依賴 `png`。
- 風險：unsafe FFI；alpha/transparency 需正確處理；需 Windows 實機驗證。

### 方案 B：原生 GDI+（windows crate，不加 crate）

`GdipCreateBitmapFromHICON` + `GdipSaveImageToStream`。

- 優點：不加外部 crate。
- 缺點/風險：GDI+ 全域 `GdiplusStartup/Shutdown` 生命週期易錯、寫錯可能崩潰；FFI 量更大。

### 方案 C：批次化 PowerShell（單次 spawn）

- 優點：低風險、可單元測。
- 缺點：仍保留 powershell 執行期依賴；執行期 cache-miss 每列仍 ~200ms。未根治。

## 4. Decision

選擇 **方案 A**。

原因：根治 spawn 成本與執行期 powershell 依賴；FFI 較 GDI+ 可控、無全域生命週期風險；新增的
`png` 是純 Rust、體積小，符合既有「避免重 crate」的 footprint 紀律（`image` 太重，不採）。

犧牲：一個新依賴 `png`；約 200 行 Windows-only unsafe FFI。

Feature flag：N/A（行為等價、輸出契約不變，不需開關）。

Migration 需求：無。磁碟 `.b64` 快取沿用；舊 powershell 寫入的快取與新路徑輸出相容（皆為 PNG base64）。

Rollback 需求：還原 `icon.rs` 至 powershell 版本並移除 `png` 依賴 + windows feature；快取格式不受影響。

## 5. Consequences

- 正面：icon 即時出現；開機 CPU 尖峰消失；少一個執行期外部相依。
- 負面/技術債：新增 `png` 依賴；多一段需 Windows 實機回歸的 unsafe 區塊。
- 對使用者：首次開面板/裝新 app 後 icon 不再逐列延遲。
- 對開發者：像素轉換（BGRA→RGBA、mask alpha）抽成純函式可單元測；FFI 編排無法單測，靠手動驗證。
- 對測試：新增像素轉換單元測；FFI 路徑列入手動驗證清單。
- 對安全：移除 `powershell -ExecutionPolicy Bypass` 子行程，攻擊面略減。

## 6. Implementation Plan（ADR 接受前不得動正式碼；本批為使用者明確指示，實作於 feature 分支）

1. `Cargo.toml`：加 `png`；windows features 加 `Win32_UI_Shell`、`Win32_Graphics_Gdi`。
2. `icon.rs`：新 `extract_shell_icon_base64` 走原生 FFI；移除 `run_icon_script` 與內嵌腳本。
3. 抽出純函式 `bgra_to_rgba(width,height,&bgra,mask)` + `encode_png_rgba` 並加單元測。
4. `warm_*` 系列改呼叫新函式（介面不變）。

## 7. Rollback Plan

- 還原 `icon.rs`、移除 `png` 與新 windows features。
- 快取 `.b64` 無需清理（格式相容）。
- 驗證：build + icon 正常顯示即回滾成功。

## 8. Validation Plan

| 測試類型          | 覆蓋目標                    | 指令 / 步驟                          |
| ----------------- | --------------------------- | ------------------------------------ |
| Unit test         | BGRA→RGBA、mask alpha、PNG  | `cargo test`                         |
| Build/lint        | FFI 編譯、無 warning        | `cargo clippy -- -D warnings`        |
| Manual validation | icon 實際顯示 + 透明度      | 刪 `%LOCALAPPDATA%\keynova\icons`，啟動，確認 app/file/folder icon 正確、透明背景無黑邊、開機無尖峰 |

效能指標：cold-cache 首次開面板 icon 到齊時間（目標：由「逐列 ~200ms×N」降到即時）。

## 9. Open Questions

- [ ] 部分舊式 icon（無 alpha 通道）是否需 mask 補 alpha——實機確認透明度。
- [ ] 取大圖示（32x32）是否足夠，或需 `SHGFI_SYSICONINDEX` + 系統 image list 取更高解析度（先不做）。
