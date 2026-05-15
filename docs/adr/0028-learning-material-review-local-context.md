# Learning Material Review — Local Context Scanning Boundary

**狀態：** 提議  
**日期：** 2026-05-15  
**決策者：** 開發者  
**相關文件：**
- docs/security.md
- docs/architecture.md
- docs/tasks/backlog.md (FEAT.11)

---

## 1. Context（技術背景）

Keynova 的 agent 已具備 `keynova.search`、`filesystem.search`、`filesystem.read` 等工具，
但缺乏對使用者本機學習資料（筆記、報告、簡報、憑證、專案）進行有組織盤點的能力。

目前問題：
- 使用者無法讓 agent 以受控、有邊界的方式列舉工作區的學習材料。
- `filesystem.search` 是無結構的索引搜尋，不支援分類或元資料彙整。
- 無法從掃描結果直接建立筆記草稿或匯出報告。

## 2. Constraints（系統限制與邊界）

- 只允許掃描使用者明確配置的 workspace root 或由使用者在審批流程中指定的路徑。
- 掃描為 **元資料優先**（名稱、大小、修改時間、副檔名分類），不得遞迴讀取文件內容。
- 預覽功能（FEAT.11.F）有嚴格 byte 上限（預設 4096 bytes）且必須通過 secret redaction。
- 未啟用 `agent.local_context.enabled = true` 時，所有掃描功能回傳錯誤。
- 掃描最多 `max_scan_files`（預設 500）個項目，深度上限 `max_depth`（預設 4）層。
- 嚴格防止 symlink 逃逸：所有路徑都必須經過 `canonicalize()` 並驗證前綴在允許的 root 之內。

## 3. Alternatives Considered（替代方案分析）

### 方案 A：直接擴充 filesystem.search（本案不採用）

**優點：** 不需要新 handler。  
**缺點：** filesystem.search 是索引型搜尋，不適合元資料分類；混合用途使 API 模糊。  
**風險：** 難以控制掃描範圍與分類邏輯。

### 方案 B：獨立 LearningMaterialManager + approval-gated tool（採用）

**優點：**
- 明確的安全邊界（enabled 旗標 + approved roots + denylist）。
- 分類邏輯集中在一個模組，易測試。
- approval-gated 確保 agent 不能在未經使用者確認的情況下掃描。

**缺點：**
- 增加新模組與 handler，提高維護成本。

**風險：** 若 canonicalize 未正確實作，存在 symlink escape 風險 → 已納入測試。

## 4. Decision（最終決策）

選擇：**方案 B**

原因：
- 安全邊界明確，符合 docs/security.md §3 路徑安全規則。
- approval-gated 工具符合 ADR-022 的 Agent Approval Boundary。
- 元資料優先掃描符合低記憶體背景模式政策（PERF.1）。

Feature flag：`agent.local_context.enabled`（預設 `false`，需使用者手動啟用）。

Rollback：刪除 `learning_material_manager.rs`、`handlers/learning_material.rs`、
移除 `state.rs` 中的 handler 註冊、移除 `settings_schema.rs` 新增的設定項。

## 5. Consequences（系統影響與副作用）

### 正面影響
- 使用者可在 agent 引導下，以受控方式盤點工作區學習材料。
- 可直接從報告建立筆記草稿。

### 負面影響 / 技術債
- 新增 `agent.local_context.*` 設定群組，Setting 面板多 5 個項目。
- 掃描邏輯使用 `std::fs::read_dir` 遞迴，需要明確測試邊界案例。

### 對安全性的影響
- 新增 **使用者可控制路徑的遞迴讀取** → 必須嚴格執行 canonicalize + root prefix 檢查。
- 所有 agent 呼叫此工具都需要使用者 approval（`AgentToolApprovalPolicy::Required`）。
- `agent.local_context.enabled = false` 作為預設值確保最小曝露面。

## 6. Implementation Plan（實作計畫）

1. FEAT.11.A: 建立本 ADR（提議狀態）。
2. FEAT.11.B: 在 `settings_schema.rs` 和 `default_config.toml` 加入 `agent.local_context.*` 設定。
3. FEAT.11.C: `models/learning_material.rs` — `MaterialClass`, `MaterialCandidate`, `ScanStats`, `ReviewReport`。
4. FEAT.11.D+E+F: `managers/learning_material_manager.rs` — `LearningMaterialManager` 含掃描、分類、安全預覽。
5. FEAT.11.G: `handlers/learning_material.rs`（IPC）；在 `agent_runtime.rs` 註冊工具；在 `handlers/agent/mod.rs` 新增 dispatch。
6. FEAT.11.H: `src/components/LearningMaterialPanel.tsx`。
7. FEAT.11.I: `export_note` / `export_markdown` 命令。
8. FEAT.11.J: 回歸測試（denied root, symlink escape, secret filter, budget, classifier）。

## 7. Rollback Plan（回滾策略）

- 設定 `agent.local_context.enabled = false`（預設），功能立即停用。
- 完整移除：刪除 3 個新檔案，還原 4 個修改檔案的對應段落。

## 8. Validation Plan（驗證方式）

| 測試類型 | 覆蓋目標 | 指令 |
|---------|---------|------|
| Unit | denied root, symlink escape, denylist, classifier | `cargo test learning_material` |
| Unit | secret filter, preview byte cap | `cargo test learning_material` |
| Unit | report markdown formatting | `cargo test learning_material` |
| Security | 非 workspace root 路徑拒絕 | 手動 + 測試 |
| Manual | Setting 面板顯示新設定 | 手動 |
| Manual | Agent 觸發工具需要 approval | 手動 |

## 9. Open Questions（未解問題）

- [ ] 是否需要支援 `scan_roots` 的 glob pattern（例如 `~/Documents/**`）？
- [ ] 深度上限 4 層是否足夠，或需要使用者可配置？