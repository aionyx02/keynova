---
type: testing_policy
status: active
priority: p1
updated: 2026-05-15
context_policy: retrieve_when_debugging
owner: project
---

# Keynova 測試策略

> Retrieval policy: 寫測試、查失敗、驗證 regression 時讀取對應章節，不需每次全讀。

**版本：** 1.0  
**最後更新：** 2026-05-13  
**相關文件：** `docs/architecture.md`, `docs/security.md`

---

## 1. 測試原則

- 測試應驗證**行為**，不驗證實作細節。
- 不得為了通過測試而修改測試邏輯或刪除測試。
- 不得在正式程式碼中加入只為測試而存在的 `pub` interface。
- 測試中不得出現 token、API key、使用者路徑等敏感資料。
- 每次修改核心行為、資料格式、安全邊界、public API，必須新增或更新對應測試。

---

## 2. 測試指令

### 2.1 Rust 後端

```bash
# 單元測試（全部）
cargo test

# 整合測試
cargo test --test integration

# 特定模組測試
cargo test --package keynova-lib -- managers::search_manager

# 包含輸出（除錯用）
cargo test -- --nocapture

# Clippy lint（等同 CI）
cargo clippy -- -D warnings

# 格式檢查
cargo fmt --check
```

### 2.2 React 前端

```bash
# 單元測試
npm run test

# 監看模式
npm run test -- --watch

# 覆蓋率報告
npm run test -- --coverage

# ESLint
npm run lint

# TypeScript 型別檢查
npx tsc --noEmit
```

### 2.3 端對端 / 手動驗證

```bash
# 開發模式（含 hot reload）
npm run tauri dev

# 正式 build
npm run tauri build
```

---

## 3. 測試類型與適用情境

| 測試類型 | 適用情境 | 工具 |
|---------|---------|------|
| unit test | 純函式、資料轉換、演算法 | `cargo test` / Vitest |
| integration test | 多模組協作、I/O、SQLite、IPC | `cargo test --test integration` |
| regression test | 修復已知 bug，防止再現 | 同上 |
| security test | 路徑遍歷、敏感資料、權限邊界 | 手動 + 程式碼審查 |
| performance test | 搜尋、索引、大量資料批次處理 | 手動量測 Task Manager / cargo bench |
| migration test | knowledge.db schema 升級降級 | `cargo test -- migration` |
| manual validation | 端對端 UX、鍵盤流程、視窗行為 | 開發者手動執行 |

---

## 4. 覆蓋率目標

| 模組 | 目標覆蓋率 | 說明 |
|------|-----------|------|
| `core/command_router.rs` | 90%+ | dispatch 邏輯是核心路徑 |
| `core/event_bus.rs` | 80%+ | publish / subscribe |
| `core/knowledge_store.rs` | 70%+ | SQLite actor，含 schema migration |
| `managers/*_manager.rs` | 70%+ | 各 manager 主要方法 |
| `handlers/*.rs` | 60%+ | CommandHandler::execute 主要分支 |
| `app/dispatch.rs` | 70%+ | IPC 分派邏輯 |
| React components | 50%+ | 主要互動流程 |

---

## 5. 重要測試案例

### 5.1 CommandRouter

- route 格式正確時 dispatch 至對應 handler
- route 缺少 `.` 時回傳錯誤
- namespace 不存在時回傳錯誤
- 多個 handler 並存不互相干擾

### 5.2 EventBus

- publish 後 subscriber 能收到事件
- legacy_tauri_topic() 正確將 `.` 轉為 `-`
- buffer 滿時舊訊息被丟棄（broadcast channel 特性）

### 5.3 KnowledgeStore / SQLite

- schema version 升級（v1→v2→v3）不遺失資料
- action_log 寫入後可查詢
- agent_audit 寫入後可查詢
- 關閉前 flush 確保資料落地

### 5.4 ConfigManager

- TOML 讀取與解析
- diff() 正確找出變更的 key
- reload_from_disk() 失敗時不覆蓋原有設定

### 5.5 SearchManager / TantivyIndex

- 索引建立後可查詢（中英文）
- rebuild 不遺失既有結果
- 搜尋結果依 score 排序

### 5.6 PortableNvimManager

- detect_nvim：configured → PATH → portable 順序檢查
- download_nvim：下載失敗時回傳 Err（不 panic）
- 進度事件正確觸發（stage, pct）

### 5.7 Agent / ReAct

- 單步驟 thought + action + observation 流程
- 超過最大步驟數時停止並回傳 Err
- 安全 gate 攔截高風險工具呼叫
- audit log 寫入 knowledge.db

### 5.8 安全邊界（路徑）

- `..` 路徑在 sandbox 中被攔截
- symlink 不逃出 workspace root
- 使用者輸入路徑必須 canonicalize 後才使用

---

## 6. 效能測試基準

以下操作必須在下列限制內完成（開發機 i7/16GB/SSD）：

| 操作 | 目標 | 資料規模 |
|------|------|---------|
| 應用程式搜尋（launcher.list） | < 100ms | 系統安裝 200+ 應用 |
| 全文搜尋（search.query） | < 200ms | 索引 10,000 個檔案 |
| Tantivy 索引建立 | < 30s | 10,000 個檔案 |
| knowledge.db action_log 寫入 | < 5ms/筆 | 非同步，不阻塞 UI |
| 冷啟動（UI 可見） | < 200ms | — |
| 記憶體（Background Core 冷啟動） | < 100MB | 不含 WebView、LLM model、PTY |

### 6.1 記憶體量測清單（PERF.1.I）

量測工具：Windows Task Manager → Keynova.exe → Memory (Private Working Set)

| 量測點 | 步驟 | 預期上限 | 備註 |
|--------|------|---------|------|
| 冷啟動 | 啟動後等待 5 秒，查看 RSS | < 100 MB | 不開啟調色盤、不開啟終端 |
| 系統匣待機 | 視窗關閉後等待 30 秒 | < 100 MB | prewarm 與 indexer 均應完成或跳過 |
| 開啟調色盤 | 呼叫 Ctrl+K 後輸入查詢 | < 120 MB | 含 WebView 及搜尋結果渲染 |
| 執行 Agent | 執行一次 /agent 任務（含 LLM 回應） | 不設上限 | 僅記錄 delta，不含已載入 model |

量測前注意事項：
- 使用 `performance.low_memory_mode = true` 時，冷啟動應省略 prewarm 和 startup indexing。
- 若 `low_memory_mode = false`，prewarm PTY 會增加約 10–20 MB；屬預期行為。
- `ai.ollama_keep_alive = "0s"` 可讓模型在每次請求後立即卸載，大幅降低 delta。

---

## 7. 平台限制與已知問題

| 平台 | 已知限制 |
|------|---------|
| Windows | Everything SDK 測試需要安裝 Everything，CI 跳過 |
| Linux | X11 hotkey 測試需要顯示伺服器，CI 使用 Xvfb |
| macOS | Accessibility API 需要使用者授權，無法在 CI 自動測試 |
| 所有平台 | Neovim 下載測試需要網路，CI 使用 mock |

---

## 8. 測試資料與 Fixture

- 測試資料放在 `src-tauri/tests/fixtures/` 或 `src/test/fixtures/`
- 不得在 fixture 中放置真實 API key 或使用者路徑
- SQLite 測試使用 `:memory:` 資料庫，不依賴磁碟狀態
- Tantivy 測試使用 `tempdir()` 臨時目錄，測試結束自動清理

---

## 9. CI 執行清單

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
npm run lint
npx tsc --noEmit
npm run test
```

所有指令必須以 exit code 0 完成才能合併。

