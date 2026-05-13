---
type: security_policy
status: active
priority: p0
updated: 2026-05-13
context_policy: retrieve_when_planning
owner: project
---

# Keynova 安全模型

> Retrieval policy: 在涉及權限、路徑、網路、Agent 工具風險時優先讀取，平時不需整份注入。

**版本：** 1.0  
**最後更新：** 2026-05-13  
**相關文件：** `docs/architecture.md`, `docs/CLAUDE.md`

---

## 1. 安全優先級

```
安全性 > 資料正確性 > 可回滾性 > 可測試性 > 效能 > 開發速度
```

任何修改若涉及安全邊界，必須先建立 ADR 並等待開發者接受，才能修改正式程式碼。

---

## 2. 信任邊界

```
[不信任區域]
  使用者輸入（CommandPalette 文字、設定值、API payload）
  使用者提供的檔案路徑
  LLM 輸出（Agent thought/action）
  外部網路回應

[信任邊界]──────────────────────────────
  Tauri IPC（invoke 呼叫）
  cmd_dispatch payload 驗證
  path canonicalize + workspace root 檢查
  AgentObservationPolicy / 工具安全 gate

[信任區域]
  Rust backend 內部模組間呼叫
  ConfigManager 讀取的本機設定
  KnowledgeStore（SQLite）讀取結果
```

---

## 3. 檔案路徑安全

### 3.1 強制規則

- **不信任使用者輸入的路徑**：所有路徑必須經過 `canonicalize()` 才能使用。
- **防止路徑穿越**：必須驗證 canonicalized 路徑以 workspace root 或 app data dir 為前綴。
- **不跟隨 symlink 逃逸**：必須檢查 canonicalized 路徑是否仍在允許範圍內。
- **Windows drive prefix 處理**：`C:\..` 必須納入路徑穿越檢查。

### 3.2 允許的讀寫路徑

| 路徑 | 操作 | 說明 |
|------|------|------|
| `%APPDATA%\Roaming\Keynova\config.toml` | 讀/寫 | 使用者設定 |
| `%LOCALAPPDATA%\Keynova\knowledge.db` | 讀/寫 | SQLite 儲存 |
| `%LOCALAPPDATA%\Keynova\notes\` | 讀/寫 | 使用者筆記 |
| `%LOCALAPPDATA%\Keynova\search\tantivy\` | 讀/寫 | 搜尋索引 |
| `%LOCALAPPDATA%\Keynova\nvim\` | 寫（下載） | Portable Neovim |
| Workspace root（使用者設定） | 遞迴讀 | 搜尋索引掃描 |

**禁止**：讀寫系統目錄（`C:\Windows\`）、其他使用者的 home 目錄、網路磁碟（未經 ADR 允許）。

### 3.3 TOCTOU 防護

檔案操作必須在取得路徑後立即執行，不得在 check 與 use 之間有長時間間隔。對於大量檔案掃描，使用快照（snapshot）而非多次讀取。

---

## 4. 敏感資料處理

### 4.1 禁止記錄的資料

- API key、token、password、cookie、OAuth refresh token、session secret
- 使用者完整的 home 目錄路徑
- 使用者私人檔案的完整路徑
- 使用者設定中的任何密碼欄位

### 4.2 Log 遮蔽規則

若需要在 log 中顯示：
- API key → `sk-****`
- 路徑 → `/Users/***/filename`
- Token → `[REDACTED]`

### 4.3 禁止存放敏感資料的位置

- IPC error payload 的 `details` 欄位
- EventBus payload
- action_log / agent_audit
- 測試 fixture
- 搜尋索引（Tantivy）

---

## 5. 網路存取

### 5.1 目前允許的網路連線

| 目標 | 用途 | 使用者可關閉 |
|------|------|------------|
| Ollama（本機 HTTP, 預設 port 11434） | 本機 AI 推理 | 不需關閉，本機連線 |
| GitHub releases（HTTPS） | Neovim portable 下載 | 是（不設定 nvim_bin 則跳過） |
| 使用者設定的翻譯 API | 翻譯功能 | 是（不設定 API key 則停用） |

### 5.2 網路安全規則

- 所有外部 HTTPS 連線必須驗證 TLS 憑證（不得使用 `danger_accept_invalid_certs`）。
- 設定 timeout（下載：60s，API 呼叫：30s）。
- 下載後必須驗證檔案完整性（checksum 或大小驗證）。
- 不得在背景自動傳送本機檔案內容或 metadata 至外部伺服器。
- 新增網路目標必須先建立 ADR。

---

## 6. Agent 工具安全

### 6.1 AgentObservationPolicy

Agent 執行工具前，`safety.rs` 中的 `ToolPermissionGate` 必須評估：
- 工具的 `risk` 等級（low / medium / high）
- 目前的 `AgentObservationPolicy`（允許 / 拒絕哪些工具類型）

高風險工具（刪除檔案、執行 shell 命令）必須在 ADR 中明確允許，並需要使用者確認。

### 6.2 LLM 輸出不信任原則

- Agent 從 LLM 收到的 action 必須經過解析與驗證（`intent.rs`）。
- LLM 輸出中的路徑、命令不得直接執行，必須與 `ToolPermissionGate` 對照。
- Prompt injection 防護：使用者輸入不得直接插入 system prompt 的指令部分。

---

## 7. IPC 安全

### 7.1 payload 驗證

- `cmd_dispatch` 收到的 payload 必須用 `serde_json` 反序列化至預期型別。
- 型別錯誤必須回傳 `IpcError`，不得 panic。
- route 格式（`namespace.command`）必須驗證，不符合格式立即回傳錯誤。

### 7.2 前端來源信任

- Tauri WebView 載入的是本機靜態資源，不接受任何外部 URL 的 IPC 呼叫。
- CSP（Content Security Policy）必須限制 `script-src`, `connect-src`。
- 不得在 WebView 中執行 `eval()` 或動態插入 `<script>`。

---

## 8. 修改安全模型時的要求

以下變更必須先建立 ADR 並更新本文件（`docs/security.md`）：

- 新增使用者可控制路徑的讀寫、刪除、移動、遞迴掃描
- 擴大既有檔案存取範圍
- 改變 allowlist、denylist、sandbox、capability 定義
- 新增對外網路連線目標
- 新增需要系統權限的功能（Accessibility API、全域 hook）
- 改變 Agent 工具的 risk 等級或 PermissionGate 邏輯
- 新增處理使用者私人檔案的功能

---

## 9. 已知安全限制

| 項目 | 現況 | 計劃 |
|------|------|------|
| Neovim 下載 checksum 驗證 | 目前僅驗證檔案大小 | ADR 後補充 SHA256 驗證 |
| Agent shell 命令執行 | 目前高風險工具需人工審查 | 計劃加入細粒度 allowlist |
| 翻譯 API key 儲存 | 存在 config.toml（明文） | 計劃支援 OS keychain |
| CSP 設定 | 尚未完整設定 | TD.5 安全強化計劃中 |

