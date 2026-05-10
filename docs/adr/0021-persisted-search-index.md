# Persisted Search Index And Runtime Ranking

**狀態：** 接受  
**日期：** 2026-05-07  
**決策者：** 開發者  
**相關文件：**
- docs/architecture.md
- docs/adr/0006-dual-search-backend.md

---

## 1. Context（技術背景）

Tantivy 先前只作為 backend 偏好標籤，需建立真實 on-disk persisted index 以提高搜尋速度。無 persisted index 時，每次搜尋需重新掃描檔案系統，延遲不可接受。

## 2. Constraints（系統限制與邊界）

- index 目錄可由 `[search].index_dir` 設定（預設 `%LOCALAPPDATA%\Keynova\search\tantivy\`）
- rebuild index 不可阻塞 UI
- Windows rebuild path 保留 file-cache scan 為 source of truth
- Knowledge Store schema 升級需備份

## 3. Alternatives Considered（替代方案分析）

### 方案 A：Tantivy on-disk persisted index

**優點：**
- 搜尋延遲從掃描級（秒）降至索引查詢級（< 200ms）
- `auto` backend 選擇：Tantivy 有 documents 時優先

**缺點：**
- 首次建立 index 需時間（< 30s for 10K files）
- index 需手動或定期 rebuild

### 方案 B：每次即時掃描

**優點：**
- 結果永遠最新

**缺點：**
- 搜尋延遲高（大型目錄 > 5s）
- 不可接受

## 4. Decision（最終決策）

選擇：**方案 A — Tantivy on-disk persisted index**

Keynova 以 Tantivy 為 persisted on-disk search provider。Windows rebuild path 保留 file-cache scan 為 source of truth，snapshot 後寫入 Tantivy index（`name`、`path`、`kind`、folder fields）。

原因：
- 搜尋速度符合目標（< 200ms）
- index 可離線使用

犧牲：
- index 可能過時（需 rebuild）

Feature flag：`[search].index_dir`（可覆寫預設位置）

Migration 需款：Knowledge Store schema versioning（SQLite `user_version = 2`）在升級前建立 sibling `backups/` 複本

Rollback 需款：刪除 index 目錄，重新 rebuild

## 5. Consequences（系統影響與副作用）

### 正面影響
- `[search].index_dir` 可覆寫預設 index 位置
- `auto` backend 選擇：Tantivy 有 documents 時優先，否則 fallback app-cache
- 搜尋延遲 < 200ms

### 負面影響 / 技術債
- Result rows 只回 `icon_key`；selected row lazy-load icon assets 透過 `search.icon`
- Index 需定期或手動 rebuild（`/rebuild_search_index`）

### 對安全性的影響
- index 目錄在允許的 app data 路徑內，不接受外部路徑

## 6. Implementation Plan（實作計畫）

已實作完成（Phase 5）。

## 7. Rollback Plan（回滾策略）

- 刪除 index 目錄，重新 rebuild
- Knowledge Store 升級前建立 `backups/` 複本，降級時還原

## 8. Validation Plan（驗證方式）

| 測試類型 | 覆蓋目標 | 指令 |
|---------|---------|------|
| Integration test | index 建立 / 查詢 | `cargo test -- tantivy_index` |
| Performance test | 10K 檔案 rebuild < 30s | 手動量測 |
| Migration test | schema v1→v2 升級 | `cargo test -- knowledge_store_migration` |

## 9. Open Questions（未解問題）

- 無