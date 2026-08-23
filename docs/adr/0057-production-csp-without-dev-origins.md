# Production CSP Without Dev Origins Or Unused Remote Hosts

**狀態：** 提議
**日期：** 2026-08-13
**決策者：** 開發者 / AI agent
**相關文件：**

- docs/security.md（§9 已知限制、§15.1 XSS、§5 網路存取）
- docs/adr/0042-inline-image-preview.md（asset protocol 停用）
- docs/memory/sessions/2026-08-13.md

---

## 1. Context

`src-tauri/tauri.conf.json` 只有單一 `security.csp`，dev 與 production 共用。目前的
`connect-src` 是：

```
connect-src 'self' ipc: http://ipc.localhost
            http://localhost:1420 ws://localhost:1420
            https://translation.googleapis.com https://translate.googleapis.com
            https://api.anthropic.com https://api.openai.com
```

兩個問題：

1. **Dev origin 出貨**：`http://localhost:1420` / `ws://localhost:1420` 是 Vite dev
   server 與 HMR socket，只有 `tauri dev` 需要，卻寫進了打包後的 policy。使用者機器上
   任何行程都能綁 1420；一旦 renderer 被注入，CSP 不會擋下對該 origin 的連線。
2. **遠端 host 其實用不到**：稽核 `src/`（排除測試）後確認 renderer **沒有任何**
   `fetch(` / `XMLHttpRequest` / `new WebSocket` 呼叫。Anthropic、OpenAI、Google
   translation 全部由 Rust 後端發出（`handlers/model.rs`、`handlers/translation.rs`、
   `managers/ai_manager.rs`），並各自受 `core/network_policy.rs` 的 host allowlist
   約束。後端流量不經過 WebView，因此不受 `connect-src` 管轄——這四個 host 寫在
   `connect-src` 裡不提供任何功能，只是把 renderer 的可外連面撐大。
   對照組：Ollama（`localhost:11434`）本來就不在 `connect-src`，功能完全正常，正好
   證明「後端呼叫不需要 renderer 授權」。

風險等級誠實評估：這**不是**已知可利用的漏洞。§15.1 的前提條件仍然成立（無
`dangerouslySetInnerHTML`、markdown 不啟用 `rehype-raw`、`script-src 'self'`、
asset protocol 已停用），所以目前沒有可用的注入點。這是 defense-in-depth：把
「萬一 renderer 被攻陷」時的資料外流管道收掉。移除既有網路目標屬 `security.md` §8
的邊界變更，因此需要一份 ADR 而不是直接改設定。

Tauri 2 原生支援 `security.devCsp`（已在 `@tauri-apps/cli` 的 `config.schema.json`
的 `SecurityConfig` 中確認），dev 專用 policy 不需要自建建置步驟。

## 2. Decision

把 CSP 拆成兩份：

- **`security.csp`（production）**：`connect-src 'self' ipc: http://ipc.localhost`。
  移除兩個 dev origin 與四個遠端 host。其餘指令不變
  （`script-src 'self'`、`img-src 'self' data:`、`object-src`/`frame-src`/`worker-src 'none'`、
  `base-uri 'self'`、`style-src 'self' 'unsafe-inline'`）。
- **`security.devCsp`（僅 `tauri dev` 注入）**：production 那份再加回
  `http://localhost:1420 ws://localhost:1420`。

`style-src 'unsafe-inline'` **不在本 ADR 範圍**：移除它需要 React inline-style 的
nonce/hash plumbing，屬另一個變更面，維持 §9 的既有待辦。

被否決的替代方案：建置期以腳本改寫 CSP 字串（`devCsp` 已是官方機制，自建腳本多一個
會 drift 的建置步驟，無額外好處）。

未納入本 ADR、留給開發者決定的相鄰項目：`core/network_policy.rs` 的
`DEFAULT_NETWORK_ALLOWLIST` 仍含 `api.tavily.com` 與 `duckduckgo.com`，但 REF.8
已移除 web search provider，這兩個 host 在程式碼中除了預設值本身已無使用點。收掉它們
是另一次預設值變更（會影響既有使用者的 `security.network_allowlist` 覆寫語意），
需獨立決定。

## 3. Consequences

### 正面

- 打包版 renderer 的可外連目標縮到只剩 IPC 通道，外流管道從「四個雲端 API + 一個本機
  dev port」降到零。
- 出貨的 policy 開始如實描述應用實際需要的東西；未來要新增 renderer 端網路目標會是一次
  顯式的 ADR 決定，而不是沿用既有寬鬆設定。
- dev 與 prod 的差異變成設定上可見的一行，而不是靠讀者自行分辨哪些 origin 只給 dev 用。

### 負面 / 技術債

- 多一份 CSP 要維護；改動時若只改 `csp` 忘了 `devCsp`，dev 環境會先壞（fail-fast，
  比 prod 悄悄放寬好，但仍是一個新的踩點）。
- 若日後真的要讓 renderer 直接打某個雲端 API（例如串流不經後端），得先改回 CSP + 立 ADR。
  這是刻意的摩擦。
- dev 與 prod 的 CSP 不再等價，代表「dev 能跑」不再保證 prod 的 CSP 沒問題。緩解：
  release build 的手動驗證清單要含 AI／翻譯／更新檢查三條路徑。

### 對使用者

- 無感。所有外部呼叫本來就走後端，功能不變。

### 對測試

- 無單元測可覆蓋（CSP 由 WebView 強制）。驗證靠 release build 手動走查 +
  DevTools console 無 CSP violation。

### 對安全

- §9「CSP dev origin 殘留」該列在本 ADR 落地後可移除；§15.1 的殘留項同步縮減為只剩
  `style-src 'unsafe-inline'`。

## 4. Rollback

- 還原 `tauri.conf.json` 的 `security` 區塊為單一 `csp` 字串（本 ADR 前的版本），
  移除 `devCsp`。無程式碼、無資料、無 schema 相依。
- 驗證回滾成功：`npm run tauri dev` 啟動無 CSP violation、AI 與翻譯功能正常。
- 若 production 出現非預期的 CSP 阻擋（例如某個未察覺的 renderer 端呼叫），先以
  `devCsp`／`csp` 加回單一 origin 的最小修補，再補 ADR 說明該 origin 為何必要，
  不要整段回退成舊的寬鬆 policy。
