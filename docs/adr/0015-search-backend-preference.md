# Search Backend Preference 與重建入口

**狀態：** 接受  
**日期：** 2026-05-05  
**決策者：** 開發者  
**相關文件：**
- docs/architecture.md
- docs/adr/0006-dual-search-backend.md

---

## 1. Context（技術背景）

ADR-006 決定雙後端策略，但缺少明確的偏好設定機制與重建入口。使用者需要能夠指定偏好的搜尋後端，並手動觸發索引重建。

## 2. Constraints（系統限制與邊界）

- 後端選擇不能影響 IPC contract（`SearchResult` 格式不變）
- `auto` 模式需依可用性自動決定
- 索引重建不可阻塞 UI

## 3. Alternatives Considered（替代方案分析）

### 方案 A：`[search].backend` 設定 + `SearchManager::select_backend` 邏輯

**優點：**
- 使用者可明確指定偏好
- backend selection 純函式化，可用測試鎖住回退規則

**缺點：**
- 多一個 config key 需維護

### 方案 B：自動偵測，不允許使用者設定

**優點：**
- 使用者無需設定

**缺點：**
- 進階使用者無法強制指定後端

## 4. Decision（最終決策）

選擇：**方案 A — `[search].backend` 設定值**

搜尋後端以 `[search].backend` 設定表達偏好（`auto` / `everything` / `app_cache` / `tantivy`），由 `SearchManager::select_backend` 依可用性決定 active backend；`search.backend` IPC 回傳 structured debug info，`/rebuild_search_index` 先重建目前 Windows file cache。

原因：
- 進階使用者可控
- 自動模式保持零設定體驗

犧牲：
- 多一個 config key

Feature flag：`[search].backend` 設定（預設 `auto`）

Migration 需求：`default_config.toml` 與 settings schema 新增 `[search]` keys

Rollback 需求：移除 `[search]` 設定 key，改回硬編碼

## 5. Consequences（系統影響與副作用）

### 正面影響
- `auto` 讓 Windows 使用者可優先走 Everything，缺少 DLL/服務時仍有 fallback
- backend selection 純函式化後可用測試鎖住回退規則
- `CommandPalette` 顯示 search backend debug badge

### 負面影響 / 技術債
- `default_config.toml` 與 settings schema 需新增 `[search]` keys

## 6. Implementation Plan（實作計畫）

已實作完成（Phase 4）。

## 7. Rollback Plan（回滾策略）

設定 `search.backend = tantivy` 強制使用 Tantivy（不需程式碼 rollback）。

## 8. Validation Plan（驗證方式）

| 測試類型 | 覆蓋目標 | 指令 |
|---------|---------|------|
| Unit test | select_backend 邏輯 | `cargo test -- search_manager` |

## 9. Open Questions（未解問題）

- 無