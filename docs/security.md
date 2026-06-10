---
type: security_policy
status: active
priority: p0
updated: 2026-06-10
context_policy: retrieve_only
owner: project
---

## Startup Preflight Snapshot Boundary (ADR-0039 / PREFLIGHT)

Startup preflight is allowed only as a bounded local bootstrap pass. It is not
general file indexing and not a private-content ingestion feature.

Allowed local writes:

- `platform_dirs::keynova_data_dir()/bootstrap/preflight-v1.json`
- Keynova-owned directories created during bootstrap, including notes/search
  index/icon-cache roots already used by the app

Allowed probes:

- Resolve/create Keynova-owned directories
- Verify app-owned icon assets required by bootstrap surfaces
- Detect local hardware facts required by `/model_download`
- Read `ai.ollama_url` from config
- Probe the configured Ollama base URL with a short local timeout and, when
  reachable, list local Ollama models

Explicitly not allowed on the boot path:

- Recursive scanning of arbitrary user files or folders
- Remote third-party model catalog fetches
- Installer-only privileged hooks that differ from `tauri dev`
- Long-running work that blocks launcher first paint

Handling rules:

- Snapshot failures degrade to warnings/partial status instead of panicking the
  UI path.
- The snapshot file stays inside Keynova-owned local storage and must not be
  reused as a generic export or audit log.
- ADR-0039 remains `proposed` until the developer explicitly accepts it; the AI
  may implement within the approved boundary but must not change ADR status.

# Keynova 安全模型

> Retrieval policy: 在涉及權限、路徑、網路、capability 風險時優先讀取，平時不需整份注入。

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
  LLM 輸出（capability 回應）
  外部網路回應

[信任邊界]──────────────────────────────
  Tauri IPC（invoke 呼叫）
  cmd_dispatch payload 驗證
  path canonicalize + workspace root 檢查
  AgentObservationPolicy（preview 機密遮蔽）

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

| 路徑                                     | 操作       | 說明            |
| ---------------------------------------- | ---------- | --------------- |
| `%APPDATA%\Roaming\Keynova\config.toml`  | 讀/寫      | 使用者設定      |
| `%LOCALAPPDATA%\Keynova\knowledge.db`    | 讀/寫      | SQLite 儲存     |
| `%LOCALAPPDATA%\Keynova\notes\`          | 讀/寫      | 使用者筆記      |
| `%LOCALAPPDATA%\Keynova\search\tantivy\` | 讀/寫      | 搜尋索引        |
| `%LOCALAPPDATA%\Keynova\crash.log`       | 讀/寫      | 後端 panic 紀錄（ADR-0051） |
| Workspace root（使用者設定）             | 遞迴讀     | 搜尋索引掃描    |

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
- action_log / agent_audit_logs
- 測試 fixture
- 搜尋索引（Tantivy）

### 4.4 壞檔隔離（Corrupt-config quarantine, ADR-0049）

`config.toml` 解析失敗時，`ConfigManager` 會把原檔複製到同目錄的
`config.toml.corrupt-<unix_secs>` 後才回退至預設值（避免下次 `persist()` 用預設覆寫造成永久遺失）。隔離檔留在 app-owned config 目錄、與 `config.toml` 同一信任區，不擴大暴露面；不自動清理（交由使用者決定）。

### 4.5 機密 at-rest 存放（OS keychain, ADR-0041）

敏感設定（`is_sensitive_key`：`ai.api_key`、`ai.openai_api_key`、
`translation.api_key` 等）**不再以明文存於 `config.toml`**。`core/secret_store.rs`
透過 `keyring` crate 寫入 OS 憑證庫（Windows Credential Manager / macOS Keychain /
Linux secret-service）；`config.toml` 只保留 `keyring:keynova:<key>` 參照。首次升級時
`ConfigManager::migrate_plaintext_secrets_to_keychain()` 會把既有明文搬入憑證庫並從
TOML 移除；憑證庫不可用時該 secret 視為未設定（不回退寫明文）。
> **ADR 狀態：** 程式碼已落地，但 ADR-0041 在 docs 仍為 `proposed`，待開發者依
> governance §3 確認後翻為 accepted（與 ADR-0039 同模式：AI 不自行翻 accepted）。

---

## 5. 網路存取

### 5.1 目前允許的網路連線

| 目標                                 | 用途                 | 使用者可關閉                 |
| ------------------------------------ | -------------------- | ---------------------------- |
| Ollama（本機 HTTP, 預設 port 11434） | 本機 AI 推理         | 不需關閉，本機連線           |
| GitHub Releases `latest.json`（HTTPS）| 應用程式自動更新檢查（ADR-0050）| 是（未設定 `plugins.updater` 前不連線；MVP 僅檢查不自動安裝）|
| 使用者設定的翻譯 API                 | 翻譯功能             | 是（不設定 API key 則停用）  |

### 5.2 網路安全規則

- 所有外部 HTTPS 連線必須驗證 TLS 憑證（不得使用 `danger_accept_invalid_certs`）。
- 設定 timeout（下載：60s，API 呼叫：30s）。
- 下載後必須驗證檔案完整性（checksum 或大小驗證）。
- 不得在背景自動傳送本機檔案內容或 metadata 至外部伺服器。
- 新增網路目標必須先建立 ADR。

---

## 6. AI Capability 安全

> **REF.8（2026-06-09）：** 舊版 ReAct agent 工具執行模型（`ToolPermissionGate`、
> `intent.rs` action 解析、agent shell/web 工具）已隨 `handlers/agent` 一併移除（ADR-0029）。
> 現行 inline capability 為**無狀態、single-shot、不執行任何工具**：只讀取本機 grounding
> 來源、回傳文字 / copy-only 建議；任何具風險的後續動作由 UI 層持有 confirmation
> ownership（ADR-0030 risk tag contract）。

### 6.1 AgentObservationPolicy（機密遮蔽）

`AgentObservationPolicy` 不再是工具 gate，而是 `core/agent_observation.rs` 的**機密遮蔽政策**：
`core/preview::read_text_preview`（`file.preview` 與 learning material review 共用）在回傳
bounded 文字前套用 `AgentObservationPolicy { redact_secrets: true }`，遮蔽常見 secret pattern。

### 6.2 LLM 輸出不信任原則

- Capability 從 LLM 收到的輸出視為不信任資料；不得直接執行其中的路徑或命令。
- 結構化 capability（`gen_command` / `fix_error`）僅產生 **copy-only** 建議，由使用者顯式複製或 replay，後端不自動執行。
- Prompt injection 防護：使用者輸入不得直接插入 system prompt 的指令部分；grounding 來源經 visibility 過濾（`GroundingSource`/`ContextVisibility`）。

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
- 改變 capability 的 risk tag 契約（ADR-0030）或 UI confirmation ownership
- 新增處理使用者私人檔案的功能

---

## 9. 已知安全限制

| 項目                      | 現況                     | 計劃                     |
| ------------------------- | ------------------------ | ------------------------ |
| 機密 at-rest 存放         | 已存 OS keychain（`keyring`，§4.5）；明文已遷出 `config.toml` | ADR-0041 程式碼已落地，待開發者翻 accepted |
| CSP 設定                  | 已收緊：`script-src 'self'`、scoped `connect-src`、`object-src`/`frame-src`/`worker-src 'none'`、`base-uri 'self'` | 殘留 `style-src 'unsafe-inline'`；如要移除需補 nonce/hash plumbing（另立 ADR） |
| 翻譯 API key 儲存         | 已隨「機密 at-rest 存放」遷入 OS keychain | （已完成，見上）|
| 程式碼簽署 / notarization | 管線已接好但尚未簽署：CI 有 secret-gated 簽章步驟，缺憑證時優雅產 unsigned build（SmartScreen/Gatekeeper 仍警告） | 開發者補上 ADR-0048 列出的 Windows/Apple 憑證 secrets 後自動啟用 |

---

## 10. Inline Image Preview 與 Asset Protocol 移除（ADR-0042）

### 10.1 設定（目前狀態）

`src-tauri/tauri.conf.json` 的 `app.security.assetProtocol.enable = false`（不設 scope），且 `Cargo.toml` 已移除 `tauri` 的 `protocol-asset` feature。CSP `img-src` 為 `'self' data:`（不含 `asset:` / `https://asset.localhost`）。

> 歷史：早期曾啟用 `assetProtocol = { enable: true, scope: ["**"] }` 供 preview pane 以 `convertFileSrc(path)` 載入圖片。2026-06-02 安全審查將其列為最高風險（#1）：scope `["**"]` 下被攻陷的 renderer（XSS）可 `convertFileSrc(<任意路徑>)` 讀取任意本機檔案而完全不經後端命令。ADR-0042 因此全面停用 asset protocol、改為 inline 傳遞。

### 10.2 圖片預覽傳遞方式

- `file.preview` 對 `PreviewKind::Image` 由後端讀檔（上限 `MAX_INLINE_IMAGE_BYTES = 8 MiB`），回傳 base64 `data:` URL（`data:<mime>;base64,...`）；超過上限只回 metadata 並標記 `oversized: true`（不含 bytes），讓 IPC 與 renderer 記憶體有界。
- `PreviewPane.tsx` 直接把 `preview.data_url` 放進 `<img src>`，不再使用 `convertFileSrc`。
- renderer 永遠拿不到可再次載入的路徑：圖片 bytes 僅針對單一、明確被預覽的路徑由後端產生，並以不透明 bytes 交付。

### 10.3 file.preview IPC 邊界

`file.preview` 為 read-only，路徑必須通過 `trim_path` + `ensure_path_exists` 驗證；text preview 走 `core/preview::read_text_preview` 套用 `AgentObservationPolicy { redact_secrets: true }` 遮蔽常見 secret pattern；max_bytes 上限 64 KiB、max_lines 上限 2000，避免 IPC payload 過大。Image 於 8 MiB 內回傳 base64 `data:` URL，超過則僅 metadata；其他 binary 不回傳檔案內容，僅 metadata。

---

## 11. Diagnostics Export Bundle（`/diag`, ADR-0047）

### 11.1 範圍

`/diag` builtin 指令產生一份可貼到 bug report 的純文字摘要：app 版本、OS/arch、feature flags（`features.*`）、redacted config、本機資料檔（`config.toml`、`knowledge.db`、`notes/`、Tantivy 索引、preflight snapshot）的存在與大小，以及 preflight 摘要（status / source_mode / ollama_reachable / 本機模型數 / generated_at）。

### 11.2 遮蔽與邊界

- **僅消費 `ConfigManager::list_all_redacted()`** 的列（從不呼叫 `get()` / `list_all()`）；`core::diagnostics::build_report` 對 sensitive 列再次強制遮成 `********`（defense-in-depth）。
- 絕對路徑的使用者 home 前綴一律收斂為 `~` / `%USERPROFILE%`，不洩漏使用者名稱。
- 本機資料檔**只讀 metadata（存在 + 大小），絕不讀檔案內容**。
- **copy-only**：沿用既有 inline 結果的 Copy + 捲動區，無網路、無自動上傳、無寫檔、無 run/edit。
- `/diag` 為核心信任指令，**不受 feature gate 限制**。

### 11.3 擴充規則

新增欄位必須通過同一套遮蔽保證，且不得引入檔案內容或網路行為。save-to-file 不在範圍內，若日後加入需另立 ADR（新增 write-IPC + 路徑邊界）。

## 12. 後端 Crash Log（panic hook, ADR-0051）

### 12.1 範圍

啟動時安裝 process 級 `std::panic::set_hook`，把每次 panic 以**單行、已遮蔽**格式 append 到 `%LOCALAPPDATA%\Keynova\crash.log`（與 `config.toml` 同信任區），再串接前一個 hook（console/stderr 行為不變）。`/diag` 會帶出最後一筆 crash（時間戳 + 截斷訊息）供 bug report。

### 12.2 遮蔽與邊界

- panic 訊息與位置中**所有**出現的使用者 home 路徑一律替換為 `~` / `%USERPROFILE%`（非僅前綴）。
- 單筆訊息上限 500 字元；整檔上限 64 KB，超過時丟最舊的整行（rotation），不會無限成長。
- 寫檔為 best-effort：失敗一律吞掉，hook 內絕不再 panic、不阻塞關閉。
- **純本機診斷 sink，非遙測**：除非使用者自行複製 `/diag` 或該檔，內容不離開機器。無網路、無自動上傳。
- 刪除 `crash.log` 任何時候皆安全。
