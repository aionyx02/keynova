---
type: task_index
status: backlog
priority: p1
updated: 2026-05-15
context_policy: retrieve_when_planning
owner: project
tags: [feature-first, roadmap, performance, safety]
---

# Backlog Tasks

> 已完成的 task groups 已搬移至 `docs/tasks/completed.md`，此檔只列尚未動工或仍進行中的工作。
> 一個 group 達 100% `[x]` 時必須在同次更新立即搬到 `completed.md`，不可在此累積已完成項目。詳見 [`docs/CLAUDE.md` Session Close 規則](../CLAUDE.md)。

## Mainline History (see completed.md)

完成依序（詳細項目見 `completed.md`）：

1. TD.5.A minimal verify baseline.
2. PERF.1 Low Memory Background Mode.
3. TD.1 + TD.2 + TD.3 prerequisite slices.
4. PERF.2 Search Execution Bound.
5. PERF.3 Heavy Feature Lazy Runtime.
6. TD.4 alignment + TD.5 hardening.
7. P3 Context Compiler Lite.
8. FEAT.11 Learning Material Review.

## Post-FEAT.11 Phase Proposal (2026-05-15)

> Source: agent UX + 功能實用性 + keynova 整體 baseline 缺口盤點。
> 各 phase 之間互相獨立；建議從 Phase 7（agent surface）與 Phase 8（baseline utilities）平行切入投資報酬率最高。
> 標 `(ADR)` 的子任務必須先完成對應 ADR 才能進入實作；見 `docs/tasks/blocked.md`。

| Phase | Track | 焦點 | ADR 需求 |
|---|---|---|---|
| 7 | AGENT.1 / AGENT.2 / AGENT.7 | Agent panel 立即可感的 UX：streaming、markdown、cancel、approval | 否 |
| 7 | AGENT.3 | Agent 工具集從「research helper」改為「action executor」 | ADR-029 |
| 8 | UTIL.1 / UTIL.2 | Calculator++ 與 in-launcher dev utilities | 否 |
| 8 | LAUNCH.1 | Search 結果 secondary actions（reveal / copy path / file ops） | 否 |
| 8 | ONBOARD.1 | First-run tour + `?` cheatsheet + empty-state CTA | 否 |
| 9 | AGENT.4 | Agent context awareness（selection / focused app / last error） | ADR-030 |
| 9 | AGENT.5 / AGENT.6 | Macros、quick actions、long-term memory | 部分 |
| 10 | CLIP.1 | Clipboard history（baseline launcher 功能） | ADR-031 |
| 10 | SNIP.1 | Snippet / text expansion | ADR-032 |
| 10 | WIN.1 | Window switcher / Alt+Tab 替代 | ADR-033 |
| 11 | UTIL.3 | Reminder / Timer / ScheduleManager | ADR-034 |
| 12 | NOTE.1 | Daily note、templates、backlinks、tag filter | 否 |
| 12 | SYNC.1 | Settings/notes export、import、git-backed sync | ADR-036 (部分) |
| 12 | LAUNCH.2 | Workspace-aware search 與切換 | 否 |
| 13 | DEV.1 | Dev specialised utilities（gitlog、issue、port killer） | ADR-035 (部分) |
| 14 | AI.1 | Inline AI surfaces（`?` prefix、selection、terminal、note） | ADR-037 |

## AGENT.1 — Streaming & Live Feedback

Goal: 消除「按下 Send 後不知道發生什麼」的等待焦慮。Phase 7, no ADR required.

- [ ] AGENT.1.A Chat 模式接上 token streaming：`ai_manager` 已具 stream API；前端新增 `ai-stream-chunk` 事件監聽，`useAi.messages` 改為增量更新；保留 `ai-response` 作為完成事件。
- [ ] AGENT.1.B Markdown 渲染：採 `react-markdown` + `remark-gfm` + 程式碼語法高亮，套用在 Chat reply 與 Agent `final answer`；保持 `<pre>` fallback。
- [ ] AGENT.1.C Cancel 改造：(1) Agent panel cancel 按鈕在 `running` 狀態也顯示，不限 `waiting_approval`。(2) Chat 模式 `useAi.send` 加 `AbortController`；新增 `ai.cancel` IPC route 中斷 in-flight 請求。
- [ ] AGENT.1.D Friendly error card：偵測 `connection refused` / `model not found` / `401 / 403` / `timeout` / `out of memory` 並提供 actionable CTA（啟動 Ollama / 開 `/setting` / 切 provider / 重試）。錯誤分類表放 `src/utils/aiErrors.ts`。
- [ ] AGENT.1.E Elapsed timer：pending message 顯示 `Thinking… 4.2s`，每 500ms tick；超過 30s 顯示 hint 「Long running — Cancel？」。

## AGENT.2 — Message UX

Goal: 把訊息列改成可操作而非純呈現。Phase 7, no ADR required.

- [ ] AGENT.2.A 每筆 assistant 訊息 hover 顯示 `Copy` / `Regenerate` 按鈕；Copy 用 `navigator.clipboard.writeText`；Regenerate 重發上一條 user prompt。
- [ ] AGENT.2.B Textarea 自動展開：依內容長度從 1 行展到 8 行（`auto-resize-textarea` 或自寫 hook）；↑/↓ 在 input 為空時叫回 history（chat 與 agent 各自獨立歷史）。
- [ ] AGENT.2.C Audit / Sources / Memory 區塊預設折疊（`<details open={false}>`）；panel 設定加 `agent.show_audit_by_default` 切換。

## AGENT.3 — Tool Surface Rebalance (ADR-029)

Goal: 把 agent 從「research helper」改造為「action executor」，貼近 keynova 鍵盤啟動器定位。Phase 7.

- [ ] AGENT.3.A ADR-029 起草：Agent Tool Surface — Action Tools Boundary。列出新工具的 approval policy、I/O 邊界、redaction 需求、與 heuristic planned_action 的關係。狀態 `提議`。
- [ ] AGENT.3.B Deprecate `dev_cargo_test/check`、`dev_npm_build/lint`：移除直接執行；改成 `terminal.propose_command` 由 agent 提議 → approval → 送 TerminalManager。System prompt 對應更新。
- [ ] AGENT.3.C 合併 `filesystem_search` 與 `keynova_search` 為 `search.unified`（後端共用 Tantivy + SearchService）。舊 tool name 保留 alias 直到下次 minor。
- [ ] AGENT.3.D 新工具 `note.create` / `note.append`（接 NoteManager，approval-gated, ActionRisk::Low）。輸入 `title` + `body`，回傳 saved path。
- [ ] AGENT.3.E 新工具 `translation.translate`（接 TranslationManager，無 approval；遵守既有 translation provider 設定）。
- [ ] AGENT.3.F 新工具 `calculator.eval`（接 CalculatorManager，無 approval；純 local）。
- [ ] AGENT.3.G 新工具 `workspace.bookmark`（接 WorkspaceManager，approval-gated, ActionRisk::Low）：加 URL / 路徑到 current workspace bookmarks。
- [ ] AGENT.3.H 新工具 `hotkey.bind_action`（接 HotkeyManager，approval-gated, ActionRisk::Medium — 改全域熱鍵）：把已存 macro 綁定 hotkey。
- [ ] AGENT.3.I 新工具 `terminal.propose_command`（approval-gated, ActionRisk::High）：agent 提議 shell；approve 後不直接執行，而是送進 terminal panel input box，由使用者再按 Enter。
- [ ] AGENT.3.J 新工具 `automation.run_pipeline`（接 AutomationHandler，approval-gated, ActionRisk::Medium）。
- [ ] AGENT.3.K 新工具 `system.set`（接 SystemManager，approval-gated, ActionRisk::Medium）：volume / brightness / power profile。
- [ ] AGENT.3.L 把 `learning_material_review` 改名 `find_unorganized_files` 並從 ReAct system prompt 推薦清單移除；保留 IPC 與 panel。
- [ ] AGENT.3.M 更新 `AGENT.7.B` payload preview：每種新工具有專屬 human-friendly summary view。
- [ ] AGENT.3.N 補測試：新工具 schema 序列化、approval gate 整合、與 heuristic planned_action 互不衝突。

## AGENT.4 — Context Awareness (ADR-030)

Goal: 讓 agent 知道「現在的使用者」在幹嘛，而非每次從零。Phase 9.

- [ ] AGENT.4.A ADR-030：OS-Level Selection Capture（hotkey、剪貼簿暫存、不持久化、隱私 opt-in）。狀態 `提議`。
- [ ] AGENT.4.B Selection capture hotkey（預設 `Ctrl+Alt+A`）：抓 OS-level 選取文字 → 開 agent panel 並帶入 prompt 前綴。Windows / macOS / Linux 三平台實作分歧寫進 `platform/`。
- [ ] AGENT.4.C ContextBundle 加 `focused_app: { title, process_name }`。
- [ ] AGENT.4.D ContextBundle 加 `last_terminal_error: { cmd, exit_code, tail_output (bounded 1KB) }`：TerminalManager 記錄最近一次 non-zero exit。
- [ ] AGENT.4.E ContextBundle 加 `recent_palette_queries: [String; 5]`：HistoryManager 最近搜尋。
- [ ] AGENT.4.F 設定 `agent.context_awareness.*` 細粒度 opt-in（each context 可單獨關閉）。

## AGENT.5 — Macros & Quick Actions

Goal: 讓重複任務變成一次設定、後續快速重跑。Phase 9.

- [ ] AGENT.5.A Agent run 完成後加 `Save as quick action` 按鈕：將 prompt + approval defaults + 偏好 tool 鏈存成 macro（`~/.keynova/macros/<id>.toml`）。
- [ ] AGENT.5.B 新增 builtin command `ai run <macro>`：列出已存 macro，Enter 觸發；不存在時模糊匹配。
- [ ] AGENT.5.C Frequent-pattern detection：HistoryManager 偵測 7 天內 ≥3 次相似 prompt（Levenshtein 或 token 相似度），於 agent panel 顯示 `Save as quick action?` chip。
- [ ] AGENT.5.D Quick action chips：agent panel 輸入框下方常駐 4 個 chips（top-3 macro + 1 動態建議）；點擊填入 prompt 不立刻送出。
- [ ] AGENT.5.E 跟 AutomationHandler 整合：macro 可標 `kind = "pipeline"` 直接執行 pipeline 而非 prompt。

## AGENT.6 — Personalization & Memory

Goal: 跨 session 真的記得使用者偏好。Phase 9.

- [ ] AGENT.6.A 把現有但未啟用的 `long_term_memory_opt_in` 設定接上實際存取路徑（目前只在 approve 流程引用一次）。
- [ ] AGENT.6.B 新增 `~/.keynova/agent_memory.toml`：儲存 user preferences / 術語映射 / 常用路徑 / 偏好回應語言。schema 加入 `models/agent_memory.rs`。
- [ ] AGENT.6.C 每次 agent run 完成後在 UI 加「Remember this preference?」明確 opt-in 按鈕；只儲存使用者勾選的條目。
- [ ] AGENT.6.D 跨 session 自動套用：memory 注入 system prompt 與 ContextBundle.recent_actions；新增 `agent.memory show / forget` builtin。

## AGENT.7 — Approval UX

Goal: 多次 approve 變一次；payload 變人讀的。Phase 7.

- [ ] AGENT.7.A Approval 加「Approve and remember for this run」checkbox：勾選後同一 tool 後續呼叫自動同意；run-scoped 不跨 run。
- [ ] AGENT.7.B Human-friendly payload preview：每種 planned_action.kind / tool 有 `summary_view` 元件；保留 raw JSON 在 `Show raw` toggle。
- [ ] AGENT.7.C Agent runs 自動上限 FIFO 20：超過從頭丟棄並 archive 到 `knowledge_store.agent_archive`；面板顯示「Archived N runs」hint。
- [ ] AGENT.7.D Approval timeout 視覺化：剩餘時間倒數，超時自動標 `approval_timeout` 並 emit event。

## LAUNCH.1 — Secondary Actions on Search Results

Goal: 搜到結果後不只能「開啟」。Phase 8, no ADR (現有 file boundary 已含).

- [ ] LAUNCH.1.A Secondary action menu：方向鍵聚焦結果 → 按 `→` / `Tab` 開菜單；含 Reveal in Explorer / Open with… / Copy path / Copy name / Show metadata。
- [ ] LAUNCH.1.B File operations（confirm-gated）：Rename、Move、Delete、Open as text、Compute hash。Delete 走 OS recycle bin，不真刪。
- [ ] LAUNCH.1.C Preview pane：選中後右側顯示前 500 行 / 4KB 內容或檔案 metadata；image 顯示縮圖。
- [ ] LAUNCH.1.D Filter chips：依 source type 過濾（file / note / app / cmd / history）；可多選；URL state 同步。
- [ ] LAUNCH.1.E Rank explainability tooltip：hover 結果顯示「why this rank」（score breakdown：tantivy score / recency / frequency）。

## LAUNCH.2 — Workspace-Aware Search

Goal: 搜尋預設貼合當下 workspace。Phase 12.

- [ ] LAUNCH.2.A Search 預設限 current workspace；輸入 `:global` 前綴擴大。
- [ ] LAUNCH.2.B Workspace switch hotkey（預設 `Ctrl+Alt+W`）；切換時清搜尋。
- [ ] LAUNCH.2.C Per-workspace quick actions：每 workspace 自己的 macro / hotkey override（存於 workspace metadata）。

## UTIL.1 — Calculator++

Goal: 把 `cal` 從基本算術擴成日常常用單位/日期/進制計算。Phase 8, no ADR.

- [ ] UTIL.1.A 單位轉換（長度、重量、溫度、容量、時間）：解析 `100 kg to lb` 等自然輸入；用 `uom` crate 或自寫 conversion table。
- [ ] UTIL.1.B 貨幣轉換：離線 fallback 表 + 可選線上 rate（exchangerate.host / open.er-api）；timeout 2s、24h 快取；網路失敗 fallback offline。
- [ ] UTIL.1.C 日期/時間運算：`3 days ago` / `today + 90 days` / `2026/12/31 - today` / `next monday`；用 `chrono-english` 或自寫 parser。
- [ ] UTIL.1.D 進制轉換:`0xff to dec` / `bin 0b1010` / `hex 255` / `oct 100`。

## UTIL.2 — Dev Utilities

Goal: 補上 dev 日常 in-launcher 小工具，純 local computation。Phase 8, no ADR.

- [ ] UTIL.2.A `uuid` / `nanoid <length>` 生成器（uuid v4 default）。
- [ ] UTIL.2.B `pw <length> [sym|alnum|alpha]` password generator。
- [ ] UTIL.2.C `hash <md5|sha1|sha256|sha512> <text>`；text 可由 clipboard 或檔案路徑。
- [ ] UTIL.2.D `b64enc` / `b64dec` / `urlenc` / `urldec`；輸入 fallback 到 clipboard。
- [ ] UTIL.2.E `json` / `jsonm`（pretty / minify）；輸入 fallback 到 clipboard；錯誤位置高亮。
- [ ] UTIL.2.F `regex <pattern> <text>` tester：含 match groups、replace preview。
- [ ] UTIL.2.G `jwt <token>` 解 header + payload（不驗簽）；標明過期 / 即將過期。
- [ ] UTIL.2.H `color <#hex|rgb()|hsl()>` 三制互轉；顯示色塊。
- [ ] UTIL.2.I `cron <expr>` 解釋成自然語言；顯示未來 5 次觸發時間。
- [ ] UTIL.2.J `killport <port>` 找佔用 process（lsof / netstat）並 confirm 後 kill；approval-gated（destructive）。

## UTIL.3 — Reminder & Timer (ADR-034)

Goal: 補上 scheduler / notification 基礎，後續 automation 也用得到。Phase 11.

- [ ] UTIL.3.A ADR-034：ScheduleManager — local cron-like scheduler 邊界、持久化策略、跟 AutomationHandler 的職責分工。狀態 `提議`。
- [ ] UTIL.3.B Tauri notification API 接上 + 跨平台權限要求流程。
- [ ] UTIL.3.C `remind <when> <text>` builtin：解析自然時間 → 排程 → 到時觸發 notification。
- [ ] UTIL.3.D `timer <duration>` + `pomodoro` builtin；狀態列指示器（tray badge）。
- [ ] UTIL.3.E ScheduleManager 持久化（`~/.keynova/schedule.toml`）+ 啟動時 reload；missed schedule 行為（catchup / skip）。
- [ ] UTIL.3.F 跟 AutomationHandler 整合：scheduled trigger 可執行 pipeline。

## CLIP.1 — Clipboard History (ADR-031)

Goal: 補上同類 launcher baseline 但 keynova 沒有的功能。Phase 10.

- [ ] CLIP.1.A ADR-031：Clipboard Capture Boundary（捕獲範圍、敏感資料偵測、儲存格式、TTL、隱私 opt-in、與 password manager 來源的互動）。狀態 `提議`。
- [ ] CLIP.1.B ClipboardManager + OS hook：Windows AddClipboardFormatListener；macOS NSPasteboard polling；Linux X11 selection + Wayland (wl-clipboard)。
- [ ] CLIP.1.C Clipboard history panel：最近 50 筆（可配置 10–500），含時間、來源 app、可搜尋。
- [ ] CLIP.1.D Pin / Unpin 持久化；pinned 不受 history TTL 影響。
- [ ] CLIP.1.E Transforms 子選單：upper / lower / title / urlenc / urldec / b64 / json pretty / sql escape / trim。
- [ ] CLIP.1.F Image clipboard 支援：縮圖預覽、限定大小（>5MB skip）、可貼回時還原。
- [ ] CLIP.1.G Sensitive content filter：偵測 password manager 來源（process name 黑名單）、長度+熵閾值；skip 或標 `secret`。
- [ ] CLIP.1.H Global hotkey 喚起 clipboard panel（預設 `Ctrl+Alt+V`）。
- [ ] CLIP.1.I Setting：`clipboard.enabled` / `history_size` / `ignore_apps` / `pause_on_focus_app` / `image_capture`。
- [ ] CLIP.1.J 加密：history 在磁碟上用 OS keyring 衍生 key 加密；記憶體中也避免常駐明文（zeroize on drop）。

## SNIP.1 — Snippet / Text Expansion (ADR-032)

Goal: 寫信、寫程式高頻場景的 text macro。Phase 10.

- [ ] SNIP.1.A ADR-032：Text Expansion Trigger Boundary（鍵盤 hook 範圍、是否監聽全域輸入、隱私邊界、不監聽密碼欄位、與 OS IME 衝突處理）。狀態 `提議`。
- [ ] SNIP.1.B SnippetManager + 儲存（`~/.keynova/snippets.toml`，含 `trigger` / `body` / `scope` / `placeholders`）。
- [ ] SNIP.1.C 階段一：launcher 內展開（打 `;trigger` 顯示候選 → Enter 展開到 clipboard，由使用者貼到目標）。
- [ ] SNIP.1.D 階段二（高風險，需 ADR 加章節）：全域鍵盤 hook 觸發，僅在 opt-in 後啟用；password field 偵測。
- [ ] SNIP.1.E Placeholder 機制：`{{date}}` / `{{clipboard}}` / `{{cursor}}` / `{{ask:name}}`；後者開 mini prompt 收集值。
- [ ] SNIP.1.F Snippet 編輯 panel（create / edit / import / export）；支援 `.toml` bundle import。
- [ ] SNIP.1.G Note 整合：note 加 frontmatter `snippet: true` 即變可觸發 snippet。

## WIN.1 — Window Switcher (ADR-033)

Goal: Alt+Tab 替代，鍵盤啟動器經典場景。Phase 10.

- [ ] WIN.1.A ADR-033：Window Enumeration Boundary（各平台 API、權限、Accessibility 需求、安全考量）。狀態 `提議`。
- [ ] WIN.1.B 平台 window enumerator：Windows EnumWindows + GetWindowText；macOS NSWorkspace + Accessibility（需引導使用者授權）；Linux wmctrl + Wayland fallback。
- [ ] WIN.1.C `win` builtin command：列所有可見視窗（含 app icon），Enter focus；fuzzy match。
- [ ] WIN.1.D Global hotkey 直接喚起 window switcher（不經 launcher 主面板）。
- [ ] WIN.1.E Workspace integration：每 workspace 顯示對應視窗（依 process / app name 過濾）。

## NOTE.1 — Notes Daily Driver Upgrades

Goal: 讓 NoteManager 從簡單檔案儲存升級為日常 driver。Phase 12, no ADR.

- [ ] NOTE.1.A `note today` / `note yesterday`：自動建立 `YYYY-MM-DD.md`；支援 daily template。
- [ ] NOTE.1.B Templates 系統：`~/.keynova/note_templates/`；建 note 時可選 template；變數展開（同 SNIP.1.E）。
- [ ] NOTE.1.C Backlinks：偵測 `[[wiki link]]` 維護反向索引（用 Tantivy 或獨立 sqlite）；NoteEditor 顯示 inbound 連結。
- [ ] NOTE.1.D Tags / frontmatter 過濾：主 launcher 搜尋加 `tag:foo` filter；frontmatter `tags: [a, b]` 索引化。
- [ ] NOTE.1.E Note 全文搜尋接到主 launcher（不只 `/note search`）：source_type 加 `note`，與 file 並列出現。
- [ ] NOTE.1.F 進階匯出：HTML / PDF（pdf 走 Tauri webview print API）；批次匯出整 workspace 為 zip。

## ONBOARD.1 — First-Run Experience

Goal: 降低學習曲線，提高功能可發現性。Phase 8, no ADR.

- [ ] ONBOARD.1.A 初次啟動 onboarding tour（4 步：開 launcher / 搜尋 / 指令 / hotkey）；可隨時 `/onboard` 重看。
- [ ] ONBOARD.1.B `?` cheatsheet overlay：任一 panel 按 `?` 顯示當前可用按鍵；資料來源 panel 自註冊。
- [ ] ONBOARD.1.C Empty state CTA：搜不到時建議「建立 note：X」/「`/help`」/「`/setting`」。
- [ ] ONBOARD.1.D Re-engage prompt：偵測 30 天未用某功能且使用 ≥7 天 → 主面板輕量提示 1 次。
- [ ] ONBOARD.1.E First-run hotkey 衝突偵測 + 修正引導：列出 conflict、提供建議替代。

## SYNC.1 — Settings/Notes Export & Sync (ADR-036, partial)

Goal: 換機器不被綁。Phase 12.

- [ ] SYNC.1.A `setting export <path>` builtin：將 settings + hotkeys + workspaces 打包 zip（不含 secret 欄位）。
- [ ] SYNC.1.B `setting import <path>` builtin：merge / overwrite 兩模式 + dry-run preview。
- [ ] SYNC.1.C Per-section 選擇性 export：CLI flag / panel checkbox。
- [ ] SYNC.1.D ADR-036：Git-Backed Sync Boundary（路徑、git binary 偵測、衝突解決、是否內嵌 git2 還是依賴 system git）。狀態 `提議`。
- [ ] SYNC.1.E Git-backed sync 實作：opt-in 設定 sync repo path；`sync push` / `sync pull` builtin；commit message 自動帶 timestamp。
- [ ] SYNC.1.F Notes sync 並支援 conflict 偵測：雙方 timestamp + content hash；衝突時保留 `note.conflict.YYYYMMDD.md`。

## DEV.1 — Developer Utilities

Goal: 補上 dev 場景貼合的小工具。Phase 13.

- [ ] DEV.1.A `gitlog <query>`：索引 git log message 與 diff 內容到 Tantivy；in-launcher 全文搜尋；點擊跳對應 commit。
- [ ] DEV.1.B Recent IDE projects 接入 launcher：解析 VS Code `storage.json` / RustRover `recentProjects.xml`；加為 source type。
- [ ] DEV.1.C ADR-035：External Provider Auth Boundary（GitHub / GitLab token 儲存、OS keyring 整合、scope 最小化）。狀態 `提議`。
- [ ] DEV.1.D `issue create` builtin：模板 + body，呼叫 GitHub / GitLab API；token 從 OS keyring 取。
- [ ] DEV.1.E Docker context switch：列 `docker context ls` 並 `dctx use <name>`；同 logic 給 kubectl context。
- [ ] DEV.1.F `pkg <add|search> <name>`：依 current workspace 判斷 cargo / npm / pip / go；建議 → confirm → terminal panel 執行。

## AI.1 — Inline AI Surfaces (ADR-037)

Goal: AI 從獨立 panel 滲透到 launcher / terminal / note。Phase 14.

- [ ] AI.1.A ADR-037：Inline AI Surfaces Boundary（觸發點、context 範圍、approval gate 改動、與 ai.chat 配額互動）。狀態 `提議`。
- [ ] AI.1.B Launcher 主框輸入 `? <問題>` 觸發 inline AI 浮層：不切 panel；浮層內可繼續追問或 `Open in AI panel`。
- [ ] AI.1.C Selection AI floating menu：選文字 + hotkey → 浮現 4 個動作（explain / translate / rewrite / find docs）；走 ai.chat 一次性對話。
- [ ] AI.1.D Terminal 失敗自動偵測：non-zero exit → 顯示「Explain this error?」inline 提示；點擊 → ai.chat 帶錯誤輸出 + 命令。
- [ ] AI.1.E Note editor 選文字 → AI 改寫 / 摘要 / 翻譯；結果可 `Replace` / `Insert below` / `Open in AI`。