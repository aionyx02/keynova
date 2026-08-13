---
type: security_policy
status: active
priority: p0
updated: 2026-08-13
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

**版本：** 1.1  
**最後更新：** 2026-08-13  
**相關文件：** `docs/architecture.md`, `docs/CLAUDE.md`

> §1–§12 描述**安全模型**（邊界是什麼）。§13–§17 是**驗證手冊**（怎麼證明邊界還在），
> 供例行稽核、PR review 與依賴變更時逐條走。

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
| 雲端 AI provider（`api.anthropic.com` / `api.openai.com` 或使用者指定的 OpenAI 相容端點） | 非本機模型推理 | 是（不設定 API key / base URL 則不連線）|

**外連一律由 Rust 後端發出**：renderer 端沒有任何 `fetch` / `XMLHttpRequest` /
WebSocket 呼叫，所以上表的目標都不需要（也不應該）出現在 WebView 的 `connect-src`
（見 ADR-0057）。

### 5.1a 後端 outbound allowlist（`core/network_policy.rs`）

所有可由設定改動的外連 URL 都必須通過 `enforce_outbound_url`：

- scheme 僅接受 `http` / `https`；`http` 只允許 loopback（`localhost`、`127.0.0.1`、`::1`），
  其餘一律要求 HTTPS。
- 非 loopback 的 host 必須落在 `security.network_allowlist`（未設定時用
  `DEFAULT_NETWORK_ALLOWLIST`）。不在清單內回錯誤，不發請求。
- 呼叫點：`handlers/model.rs`（Ollama / OpenAI base URL、Anthropic endpoint）、
  `handlers/translation.rs`、`managers/ai_manager.rs`、`core/startup_preflight.rs`。
- 現行預設清單含 `api.tavily.com` 與 `duckduckgo.com`，但 REF.8 已移除 web search
  provider —— 屬殘留的死授權，列於 §9 待開發者決定是否收掉。

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
| CSP `connect-src` 過寬       | production CSP 仍含 dev origin `localhost:1420` 與四個遠端 host，但 renderer 實際不發任何直接網路請求 | ADR-0057（`提議`）：拆 `csp` / `devCsp`，production `connect-src` 收到只剩 `'self' ipc: http://ipc.localhost` |
| 後端 allowlist 殘留死授權    | `DEFAULT_NETWORK_ALLOWLIST` 仍含 `api.tavily.com`、`duckduckgo.com`，但 REF.8 已移除 web search provider | 收掉兩個 host 屬預設值變更（影響既有 `security.network_allowlist` 覆寫語意），待開發者決定 |

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

---

## 13. 依賴與供應鏈驗證（`npm view`）

### 13.1 安裝前驗證（新增或升級任何套件前必做）

```bash
npm view <pkg>                                   # maintainers / repository / license / 最新版
npm view <pkg> time.created time.modified        # 「沉睡多年突然發版」= 帳號接管訊號
npm view <pkg> maintainers dist-tags versions --json
npm view <pkg>@<ver> dist.integrity dist.tarball # 與 package-lock.json 的 integrity 對照
npm view <pkg> scripts                           # 是否帶 pre/post install 生命週期腳本
npm view <pkg> dependencies                      # 間接依賴爆炸面
```

紅旗（命中任一項 → 不安裝，先查清楚）：

- 版本發布時間過新（< 30 天）且無其他採用者，或套件長期停更後突然換 maintainer。
- 名稱與知名套件僅差一字元、大小寫或連字號（typosquatting）。
- `repository` 欄位缺失，或指向與 npm 頁面宣稱不符的 repo。
- 帶 `preinstall` / `postinstall` / `install` 腳本 —— Keynova 的直接依賴**都不應該需要**。
- `dist.integrity` 與 lockfile 既有值不一致（同版本被重新發布）。

### 13.2 專案規則

- 直接 runtime 依賴刻意保持小：`@tauri-apps/*`、`react`/`react-dom`、
  `react-markdown` + `remark-gfm` + `rehype-highlight`、`@xterm/*`、`zustand`。
  **新增任何 runtime 依賴視同 §8 的邊界變更，需先立 ADR。**
- 安裝一律 `npm ci`（走 lockfile integrity），不用 `npm install` 造成未審查的版本漂移。
- 本 repo 自有一個 lifecycle script：`prepare` → `npm run hooks:install`（安裝
  `.githooks/pre-commit`，純本機、無網路）。稽核第三方腳本時把它排除在「未知腳本」之外。
- 例行掃描已進 CI（`.github/workflows/audit.yml`，見 §17.1），本機可直接跑
  `npm audit --audit-level=high`、`npm audit signatures`、`cargo audit`。
- Baseline 演進：2026-06-10 為 0 vulnerabilities；2026-08-13 重測為 9（2 low / 7 high），
  全部是 build/test 工具鏈的間接依賴（`vite`/`esbuild`/`postcss`/`@babel/core`/
  `brace-expansion`/`js-yaml`/`nanoid`/`form-data`/`ws`），已用 `npm audit fix`
  在 semver 範圍內升級（`package.json` 版本區間未變，只動 lockfile），回到 0。
  **教訓：一次性 baseline 會過期，所以掃描必須由 CI 排程持續執行，而不是靠人記得跑。**
- Dependabot 已涵蓋 npm / cargo / github-actions 三個 ecosystem（`.github/dependabot.yml`，
  每週）。**Dependabot PR 不因為是機器人開的就免審**：同樣跑 §13.1 的 `npm view` 檢查，
  並確認 CI 綠燈才合。

---

## 14. 授權驗證是否失效（authorization bypass）

Keynova 沒有多使用者帳號系統，授權面是**本機能力授權**：Tauri capability allowlist、
IPC namespace 的 feature gate、以及高風險動作的 UI confirmation ownership。
以下任一層失效即為 P0。

### 14.1 三個強制點

| 層                       | 位置                                                         | 失效表現                       |
| ------------------------ | ------------------------------------------------------------ | ------------------------------ |
| Tauri capability allowlist | `src-tauri/capabilities/default.json`                       | 前端能呼叫未列於 permissions 的 core 指令 |
| Namespace feature gate   | `app/dispatch.rs::namespace_feature_block`                    | 已關閉的 feature 仍能經 IPC 觸發 |
| Risk / confirmation      | `UnifiedResult.ActionChip` 的 risk + `ConfirmRequirement`（ADR-0030） | 高風險動作未經使用者確認即執行 |

### 14.2 迴歸檢查清單

- [ ] `capabilities/default.json` 維持最小集合（現況：`core:default`、`opener:default`、
      window `hide`/`show`/`set-focus`/`is-visible`/`set-size`、`updater:default`）。
      新增任一條 permission 必須有 ADR。
- [ ] Gate 由**後端**強制，不是只把前端 UI 藏起來：關閉 `features.notes` 後直接
      `cmd_dispatch("note.save", …)` 必須被拒（測試 `route_feature_key_gates_feature_namespaces`）。
- [ ] Namespace 比對是**分段等值**而非 `starts_with`：`system_monitoring.*` 不得被
      `system` guard 命中（已有迴歸測試）。
- [ ] 繞道入口一律回到同一個 gate：`automation.execute`、`automation.execute_pipeline`、
      `action.run` 的 `CommandRoute` 都經由 `dispatch_command` 再分派。
      **新增任何批次／腳本執行入口不得直接呼叫 `command_router.dispatch`。**
- [ ] AI namespace（`capability`/`ai`/`agent`）刻意豁免 namespace gate、改在 handler 內
      逐 route 檢查（避免模型設定的 bootstrap 死鎖）。改動時確認 bootstrap 例外
      （`ai.check_setup`、`capability.list`）沒有被擴大成整個 namespace 免檢。
- [ ] `action_ref` 維持不透明 handle（由 `action_arena` 解析），前端不得改成直接傳路徑或
      命令字串；過期 ref 必須回 `stale_action_ref` 而不是照常執行。
- [ ] 缺 key 視為啟用的 `unwrap_or(true)` idiom **只適用於 feature 可見性**；安全開關必須
      fail-closed（缺值 = 關閉）。

---

## 15. 注入驗證：XSS 與 SQL injection

### 15.1 XSS（renderer 面）

現況（可稽核事實）：

- 全 `src/` 無 `dangerouslySetInnerHTML`、無 `eval()`、無動態 `<script>` 注入；
  `innerHTML` 僅出現在測試 teardown。
- Markdown 是唯一「不信任文字 → DOM」通道：`shared/components/MarkdownImpl.tsx` 用
  `react-markdown` + `remark-gfm` + `rehype-highlight`，**未**啟用 `rehype-raw`，
  因此模型輸出裡的原始 HTML 不會被解析成節點；連結 `href` 走 react-markdown 預設的
  `urlTransform`，`javascript:` 等 scheme 會被剝除。
- CSP（`tauri.conf.json`）：`script-src 'self'`、`object-src`/`frame-src`/`worker-src 'none'`、
  `base-uri 'self'`、`img-src 'self' data:`；asset protocol 已停用（ADR-0042），
  所以即使 renderer 被攻陷也拿不到任意本機檔案讀取原語。

檢查清單：

- [ ] 不得引入 `rehype-raw`、`dangerouslySetInnerHTML`、`innerHTML =`、`eval`、`new Function`。
      不信任來源包含：LLM 輸出、剪貼簿內容、檔名／路徑、搜尋結果、設定值。
- [ ] 檔名、路徑、搜尋 highlight 一律以 React 文字節點渲染，不得自行組 HTML 字串。
- [ ] 外部連結一律 `target="_blank" rel="noreferrer"`。
- [ ] 放寬 CSP（新增 `connect-src` 目標、放行 `script-src`）→ 先立 ADR（§8）。
- [ ] 已知殘留（§9）：`style-src 'unsafe-inline'`（需 nonce/hash plumbing，另立 ADR）；
      production `connect-src` 過寬已由 **ADR-0057（`提議`）** 涵蓋，待開發者接受後實作。

### 15.2 SQL injection（knowledge store 面）

現況：

- 所有 row 級 SQL 都是**參數化**的：`core/knowledge_store/sql.rs` 一律 `?1..?n` +
  `params![]` 綁值，沒有任何把使用者字串串進 SQL 的位置。
- 唯二的動態 SQL 在 schema 遷移：`ensure_workflow_column(conn, "succeeded", "INTEGER")` /
  `("project_root", "TEXT")`，以及 `PRAGMA user_version = {version}`（`u32`）。
  插值的都是**編譯期常數／整數**，不接受外部輸入。

檢查清單：

- [ ] 新查詢一律 `params!` / `named_params!`。
- [ ] identifier（表名、欄名）若需動態，只能取自程式內白名單常數，
      **不得**來自 IPC payload、config 或檔案內容。
- [ ] `LIKE` 樣式（例如 `/setting %` 清理）同樣走參數綁定，不做字串拼接。
- [ ] 不得為了方便而 `execute_batch(&format!(...))` 帶入任何 runtime 字串。
- [ ] 搜尋端：Tantivy query 字串屬不信任輸入，解析失敗必須回錯誤而非 panic
      （非 SQL 面，但同屬 query injection 面）。

---

## 16. Agent 信任邊界：agent 組態檔

`.mcp.json`、`.claude/settings.json`、`.cursor/*.mdc`、`.amazonq/mcp.json` 這類檔案是
**可執行的組態**：它們能讓本機 AI agent 自動啟動外部程序、放寬工具授權、或注入額外規則。
把它們放進版本控制，等於把「誰能在 clone 此 repo 的人的機器上跑什麼」寫進 repo，
屬於 §8 的信任邊界變更。

### 16.1 威脅模型

- **MCP server 投毒**：`.mcp.json` 的 `command` / `args` 會在 agent 啟動時執行，
  等同 repo 內的 `postinstall`；`env` 區塊還可能挾帶 token。
- **權限自動放行**：`.claude/settings.json` 的 `permissions` / allow-list 能把
  「需人工確認」變成自動執行，繞過 §14 的 confirmation ownership。
- **規則注入（config 形式的 prompt injection）**：`.cursor/*.mdc`、`AGENTS.md`、
  `CLAUDE.md` 會被 agent 當指令讀取。PR 改一行規則就能改變 agent 行為，
  而 diff 看起來只是「文件變更」。
- **跨工具擴散**：Claude / Cursor / Amazon Q / Windsurf 各有一份組態，稽核容易漏掉某一家。

### 16.2 現況稽核（2026-08-13）

- repo 內**不存在** `.mcp.json`、`.cursor/`、`.amazonq/`、`.vscode/`。
- `.claude/` 存在但為空目錄，且已列入 `.gitignore`（agent 本機設定不入版控）。
- `AGENTS.md` 亦已 gitignore。
- 被追蹤、且會被 agent 讀取的規則檔只有 `CLAUDE.md` 與 `docs/CLAUDE.md`
  （純文字治理規則，不含可執行組態）。

### 16.3 規則

- 預設**不追蹤**任何 agent 可執行組態。`.gitignore` 的 agent 區塊涵蓋 `.claude/`、
  `.mcp.json`、`.cursor/`、`.cursorrules`、`.amazonq/`、`.windsurf/`、`.windsurfrules`、
  `.roo/`、`.aider*`、`.github/copilot-instructions.md`、`AGENTS.md`
  （2026-08-13 補齊；已審查的例外只有 `CLAUDE.md` 與 `docs/CLAUDE.md`）。
  新增其他 agent 工具時要同步補條目——**ignore 清單漏一個，等於預設允許該工具的組態入庫**。
- 若日後確實要共用某份組態：立 ADR + 逐條說明每個 MCP server 的 `command`／權限範圍，
  並要求 code owner review。
- 組態 `env` 內不得出現實際密鑰。密鑰只走 OS keychain（§4.5）或本機 `.env`（已 gitignore）。
- clone、rebase 或接手外部 PR 前掃一次：

```bash
git ls-files | grep -Ei '(^|/)(\.mcp\.json|\.claude/|\.cursor/|\.amazonq/|\.windsurf|\.aider|\.vscode/|AGENTS\.md|CLAUDE\.md)'
```

- PR review 硬規則：凡改到 `CLAUDE.md`、`docs/CLAUDE.md` 或任何 agent 組態的 PR，
  一律當作**權限變更**審，不適用「純文件變更可快速合併」的慣例。
- AI agent 不得自行放寬自己的權限：不得修改 `.claude/`、`capabilities/default.json`
  或本節規則（對應 `docs/CLAUDE.md` §3 的 ADR 權限限制）。

---

## 17. 靜態驗證（SAST）與 CI 安全閘門

### 17.1 現行 pipeline

| 檢查        | 工具 / 位置                                                       | 觸發                              | 涵蓋                     |
| ----------- | ----------------------------------------------------------------- | --------------------------------- | ------------------------ |
| SAST        | CodeQL（`.github/workflows/codeql.yml`）：`javascript-typescript` + `rust`，`build-mode: none` | push/PR 到 `main`·`dev` + 每週一 06:17 UTC | 注入、危險 sink、常見 CWE |
| Lint (Rust) | `cargo clippy -- -D warnings`（`ci.yml`，3-OS 矩陣）                | 每次 push                         | panic 面、正確性 lint    |
| Lint (TS)   | `eslint --ext .ts,.tsx --max-warnings 0`                           | `npm run check` / pre-commit      | React/TS 危險模式        |
| 型別        | `tsc`（`npm run build`）                                           | 每次 push（CI 先建前端）          | 型別層防呆               |
| 測試        | `cargo test` + `vitest`                                            | push / `npm run verify`           | §14 授權 gate 等安全迴歸 |
| 依賴弱點    | `npm audit --audit-level=high` + `cargo audit`（`.github/workflows/audit.yml`） | push/PR 到 `main`·`dev` + 每週一 07:05 UTC + 手動 | 已發布版本的已知 CVE（**會擋合併**） |
| 依賴升級    | Dependabot（npm / cargo / github-actions，每週）                    | 排程                              | 上游新版本               |
| 文件同步    | `npm run docs:refresh` guards、`guard:size`                        | pre-commit / verify               | 安全模型與程式碼不漂移   |

### 17.2 已知缺口（待辦，不是已完成事項）

- `cargo audit` 的 CI job 尚未在真實 CI 上跑過綠燈（本機未安裝 `cargo-audit`，
  2026-08-13 只驗證了 workflow YAML 可解析與 `npm audit` 那一半）。首次 push 後需確認。
- 尚未導入 `cargo-deny`（license + 來源 + 重複依賴），目前只擋 RustSec advisory。
- `npm audit signatures` 目前為 `continue-on-error`（缺簽章多半是 registry 端缺口，
  不當作合併阻擋）；若日後要收緊需確認全依賴樹皆有簽章。
- CodeQL 對 Rust 使用 `build-mode: none`（不編譯抽取），涵蓋率低於 build 模式；
  提高涵蓋率需評估 CI 時間成本。
- GitHub secret scanning / push protection 尚無設定紀錄，建議開啟。
- 無 SBOM 產出、release artifact 未簽章（見 §9 code signing）。

### 17.3 合併前最小安全門檻

`npm run verify` 綠燈 + CodeQL 無新 alert + 觸及 §8 清單的變更有對應 ADR。
