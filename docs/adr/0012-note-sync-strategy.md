# 筆記同步策略

**狀態：** 接受  
**日期：** 2026-05-03  
**決策者：** 開發者  
**相關文件：**
- docs/architecture.md
- docs/security.md

---

## 1. Context（技術背景）

需選擇筆記儲存策略，確保簡單可靠，且不引入同步衝突問題。Keynova 筆記功能需要支援 Neovim 外部編輯，因此純記憶體方案不可行。

## 2. Constraints（系統限制與邊界）

- 需支援 Neovim 外部編輯（直接修改磁碟檔案）
- MVP 不需即時多裝置同步
- 檔名需安全（sanitize）
- 儲存目錄可由使用者設定（`notes.storage_dir`）

## 3. Alternatives Considered（替代方案分析）

### 方案 A：Markdown 檔案儲存（純 file I/O）

**優點：**
- 簡單可靠，可用任何編輯器開啟
- 無衝突（單用戶 local，單一文件單次編輯）
- 相容 Obsidian / Neovim / 任何文字編輯器

**缺點：**
- 外部編輯修改由使用者手動重新整理（MVP 不做即時監看）

### 方案 B：SQLite 儲存

**優點：**
- 搜尋效率高
- ACID 保證

**缺點：**
- 外部編輯困難（需 SQL 工具）
- 增加複雜度

### 方案 C：CRDT / OT 即時同步

**優點：**
- 支援多裝置即時同步、衝突解決

**缺點：**
- 複雜度遠超 MVP 需求
- 不在 Phase 1–3 範圍

## 4. Decision（最終決策）

選擇：**方案 A — Markdown 檔案儲存**

原因：
- 最簡單可靠，與 Neovim 外部編輯相容
- 無衝突，使用者可用任何工具管理

犧牲：
- 外部編輯後需手動重新整理（MVP）
- 全文搜尋需靠 Tantivy（而非 SQLite FTS）

Feature flag：N/A

Migration 需求：N/A（新功能）

Rollback 需求：N/A（檔案已在磁碟，無 migration 風險）

## 5. Consequences（系統影響與副作用）

### 正面影響
- `managers/note_manager.rs`：純 file I/O，filename = `sanitize(name) + ".md"`
- 前端 `NoteEditor.tsx`：auto-save（800ms debounce）+ Ctrl+S 立即儲存

### 負面影響 / 技術債
- 進階需求（OneDrive/Obsidian 整合）留在未來 Phase

### 對安全性的影響
- 使用者可設定 `notes.storage_dir`，需驗證路徑不逃出允許範圍

## 6. Implementation Plan（實作計畫）

已實作完成（Phase 3）。

## 7. Rollback Plan（回滾策略）

N/A — 檔案儲存無需 rollback，直接刪除或移動目錄即可。

## 8. Validation Plan（驗證方式）

| 測試類型 | 覆蓋目標 | 指令 |
|---------|---------|------|
| Unit test | sanitize / file I/O | `cargo test -- note_manager` |
| Security test | storage_dir 路徑驗證 | 手動 + 程式碼審查 |

## 9. Open Questions（未解問題）

- 無