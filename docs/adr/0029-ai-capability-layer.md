# AI Capability Layer — Stateless Inline Capabilities Over Search Results

**狀態：** 接受
**日期：** 2026-05-20
**決策者：** 開發者
**相關文件：**

- docs/tasks/refactor-ai-capability.md
- docs/memory/current.md
- docs/architecture.md
- docs/security.md
- docs/adr/0011-ai-provider-abstraction.md
- docs/adr/0016-agent-read-only-tools.md
- docs/adr/0022-agent-approval-boundary.md
- docs/adr/0023-react-tool-schema-foundation.md
- docs/adr/0024-agent-system-indexer.md
- docs/adr/0026-provider-driven-react-loop.md

---

## 1. Context（技術背景）

Keynova 的核心定位是「鍵盤先行的工作流啟動器」，但目前 AI 軌道（chat-first `AiPanel`、autonomous multi-step ReAct loop）在使用者實際路徑上偏離了 launcher hotpath：

- `AiPanel` 與 `CommandPalette` 並存兩條結果消化路徑，導致 UI 雙焦點與快捷鍵歧義。
- ReAct loop 設計在背景由 LLM 多步驟驅動工具，與「結果列上一鍵動作」的鍵盤工作流節奏不一致。
- 後端持有 approval state 並回推 UI 等待，使 UI 對風險判斷的所有權模糊。
- `CommandPalette.tsx`（1582 行）、`handlers/agent/mod.rs`（2406 行）、`agent_runtime.rs`（1891 行）、`AiPanel.tsx`（979 行）已超過健康閾值，且大部分行數來自 chat/agent 軌道，使搜尋熱路徑難以演進。

若不處理，將：

- 阻塞「unified search 為產品脊椎」的策略落地（見 `docs/memory/current.md`）。
- 累積使用者層級的 UX 不一致（兩條 AI 觸發路徑）。
- 累積程式碼健康債：熱路徑檔案肥大、agent module 邊界混亂。

本 ADR 是 REF.0 的決策載體；REF.1–REF.8 等待本 ADR 被開發者接受後才開始實作（見 `docs/tasks/blocked.md` ADR-0029 Refactor Gate）。

## 2. Constraints（系統限制與邊界）

- 平台：Windows 10+ / macOS 11+ / Linux（X11/Wayland），不引入新作業系統權限邊界。
- 既有 launcher hotpath 效能不得回歸；目標仍為 cold open < 200ms、warm open < 50ms、search first chunk P50 < 80ms（細節見 `refactor-ai-capability.md` Validation Matrix）。
- 至少一個 release cycle 必須保留 chat-first 行為作為相容回退（`ai.legacy_agent` 旗標）。
- 不引入新的網路、檔案系統、shell 邊界；遵守 `docs/security.md` 的路徑、審批、敏感資料規則與 ADR-0022 approval boundary。
- AI capability 必須能在本機 Ollama 後端與 `openai-compatible` 後端同形可用（延續 ADR-0011 / 0026 抽象）。
- 不擴大 `agent_runtime` 工具暴露面；REF.4 的初版 capability 限於 `explain`、`summarize`、`fix_error` 三項。
- 文件守則：本 ADR 與配套修改需通過 `npm run docs:refresh` 的 guard suite，不得降低守則門檻。

## 3. Alternatives Considered（替代方案分析）

### 方案 A：維持 chat-first `AiPanel` + multi-step ReAct（現狀延伸）

**優點：**

- 不需任何結構性重構，短期工時最低。
- 對既有 agent 工具生態保留最大延展空間。

**缺點：**

- UI 雙焦點與快捷鍵歧義無法收斂。
- `CommandPalette` 熱路徑檔案持續肥大化。
- ReAct loop 與 launcher 一鍵節奏錯位的根因不解。

**效能分析：** 啟動成本與 idle RSS 預期持平；UI 響應仍受 panel 雙路徑競爭影響。

**風險：** 策略上違背 `docs/memory/current.md`「AI 為能力層」的方向；長期維護成本最高。

### 方案 B：AI 降為 stateless capability layer，由結果列 `ActionChip` 觸發（採用）

**優點：**

- 把 unified search 立為產品脊椎，AI 顯式變成結果上的「能力」而非平行 UI。
- single-step stateless call 使 capability 易測試、易封裝、易快取。
- UI-owned confirmation 把風險判斷集中在使用者可見的觸發點，後端只需回報 risk tag。
- 與計畫中的 `UnifiedResult`（REF.1）與 workflow memory（REF.5）天然契合。

**缺點：**

- 需要新增 `core/ai_capability/` 與配套前端 hooks/UI；近期工時集中。
- 觀察期內 chat-first code path 需共存，會短暫膨脹一段時間（由 REF.7/REF.8 收尾）。

**效能分析：**

- 時間複雜度：capability call 為 single-step，無多步 reasoning，預期 inline P50 在 Ollama `qwen2.5:7b` 下 < 800ms。
- 空間複雜度：移除 chat session memory 後降低。
- 啟動成本：背景不再預掛 ReAct planner，cold open 受益。
- 長期維護成本：模組邊界更小、行數收斂，後續迭代成本下降。

**風險：** 觀察期 dual code path 期間若出現 P0 回歸，REF.7 的預設切換需延後（已納入 Rollback）。

### 方案 C：完全移除 AI 軌道（不採用）

**優點：** 程式碼面最簡。
**缺點：** `explain` / `summarize` / `fix_error` 是 launcher 對開發者實際有價值的加值；完全移除會喪失差異化。
**風險：** 與 `current.md`「AI 為能力層」的方向直接抵觸。

## 4. Decision（最終決策）

選擇：**方案 B — AI capability layer**

具體聲明：

- AI 從產品核心移至 capability layer；結果列以 `ActionChip` 為一級觸發。
- chat-first `AiPanel` 從目標架構移除，於觀察期後依 REF.8 物理刪除。
- autonomous multi-step ReAct 由 single-step stateless capability call 取代。**Capability 內部嚴格 single-shot**：capability 不得呼叫其他 capability；UI 層可由使用者操作驅動連續 `ActionChip` 觸發（user-driven chaining 允許，函式內鏈式禁止）。為避免誤觸與重複呼叫，**chip 在所屬 capability call in-flight 期間必須 disabled（state-based，非 time-based debounce）**，由 REF.6 落實；不引入 chip telemetry（如 REF.7 量化 gate 需要使用率資料再另行評估）。
- 後端 approval state 改為 UI-owned confirmation over backend risk tags。**risk tag 的最小資料契約另立 ADR-0030 規範**（`{requires_confirmation, reason}` 與 fail-safe-to-confirm 規則，刻意不含 category × severity 等過早抽象）；本 ADR 僅承諾「後端輸出風險標註、UI 持有確認權」的責任歸屬。
- 初版 capability：`explain`、`summarize`、`fix_error`。其中 `explain` / `summarize` 為純文字輸出，`requires_confirmation = false`；`fix_error` 由 capability 內部依當下動作分流——純解釋型（只顯示 AI 對錯誤的說明）`false`；觸碰檔案 / 執行命令的修復路徑 `true`。`suggest_next`（工作流記憶配套）等待 REF.5 schema 穩定後再評估。
- **`context_hash` 不為 capability 一級必填參數**；於 capability schema 中作為 optional 欄位，由叫用端視需要填入。初版三項 capability 不消費此欄位，未來 workflow-aware capability（如 `suggest_next`）可在不修改契約的前提下啟用。
- **`agent_runtime.rs` 在 REF.8 採兩步處理**：先抽出純審計 / 歷史查詢邏輯到獨立模組 `core/agent_audit.rs`（**作用域嚴格為「legacy agent approval audit 歷史紀錄查詢」**；新 capability 的 audit 由其 metadata 各自分流，不混入此模組），再把剩餘的 ReAct loop 與 chat-first dispatch 完全刪除。確保 ADR-0022 audit 紀錄不會隨 ReAct loop 消失成為孤兒。

原因：唯一同時滿足「鍵盤先行」「unified search 為脊椎」「程式碼健康收斂」三個目標的方案。

犧牲：觀察期程式碼短期膨脹；近期工時集中於 REF.1–REF.4。

Feature flag：`ai.legacy_agent`

- 觀察期預設 `true`（保留 chat-first 行為與舊 ReAct loop）。
- REF.7 切換為 `false`；REF.8 移除旗標與對應程式碼。
- **觀察窗口長度**：「一個 minor release tag」與「14 個日曆天」兩者較長者，且觀察期內無 P0 回歸報告才視為通過。任一條件未達成則延後預設切換到下一個觀察窗口。

Migration 需求：

- 新增 `UnifiedResult` schema（REF.1，獨立交付）。
- 新增 knowledge DB schema v4 `workflow_history` 表（REF.5，附 rollback 指南）。
- 既有 `SearchResult` / `BuiltinCommandResult` 在觀察期維持為 deprecated 相容型別。
- 後端 risk tag schema 由 ADR-0030 定義並與 REF.4 一同落地。

Rollback 需求：見 §7。

## 5. Consequences（系統影響與副作用）

### 正面影響

- `CommandPalette` 熱路徑收斂；REF.7 目標 < 250 行。
- agent handler 行數目標 < 600 行；`agent_runtime.rs` < 400 行直至 REF.8 刪除。
- UI 觸發點單一化，鍵盤工作流連貫。
- capability 為 single-step，可被快取與離線降級。

### 負面影響 / 技術債

- 觀察期 dual path 共存，短期程式碼面與測試面膨脹。
- 受影響的 ADR 與性質如下表；本 ADR 僅標記受影響，正式狀態變更（廢棄 / 取代）延後到 REF.8 並由開發者操作：

| 受影響 ADR                           | 影響性質                                                   | REF.8 預期動作                |
| ------------------------------------ | ---------------------------------------------------------- | ----------------------------- |
| 0011 AI Provider Abstraction         | 仍適用（capability 共用 provider trait）                   | 不動，補一行 cross-ref        |
| 0016 Agent Read-only Tools           | 仍適用，但 capability 不再透過 agent runtime 觸發          | 視 REF.8 結果決定要不要 amend |
| 0022 Agent Approval Boundary         | 部分被取代（approval ownership 移到 UI），audit 部分仍適用 | REF.8 必須 amend              |
| 0023 ReAct Tool Schema Foundation    | 被取代（capability 不走 ReAct schema）                     | REF.8 標為 superseded by 0029 |
| 0024 Agent SystemIndexer Search Path | 仍適用（capability 可呼叫既有 indexer）                    | 不動                          |
| 0026 Provider-Driven ReAct Loop      | 被取代（ReAct loop 整段移除）                              | REF.8 標為 superseded by 0029 |
| 0037（預留）Inline AI Surfaces       | 被取代，自始不寫                                           | `decisions.md` 刪除預留列     |

### 對使用者的影響

- chat-first AI 體驗在觀察期透過 `ai.legacy_agent = true` 保留；REF.7 後預設下線、REF.8 後物理移除。
- AI 動作改由結果列 `ActionChip` 觸發，鍵盤節奏更一致。
- 高風險動作的求證點轉由 UI 顯示。

### 對開發者的影響

- 需熟悉 `core/ai_capability/` 註冊機制與 `ActionChip` 契約。
- 既有 agent 工具相關修改需區分「保留至 REF.8」與「直接移植到 capability」。

### 對測試的影響

- 新增 capability 級別 unit / integration 覆蓋。
- 觀察期需保留 chat / ReAct 回歸測試直到 REF.8。
- Bug A 啟動聚焦/IME 競賽與 Bug B 刪除驗證流程在 REF.2 後、REF.7 前各跑一次手動回歸。

### 對安全性的影響

- approval ownership 由後端轉向 UI；後端仍負責產生 risk tag 與審計紀錄（延續 ADR-0022 的審計範疇）。
- single-step capability 移除 multi-step prompt 對 tool surface 的累積暴露。
- 不擴大檔案、網路、shell 邊界；ADR-0027 generic shell sandbox 限制不變。

## 6. Implementation Plan（實作計畫）

**此 ADR 未被開發者接受前，不得修改 runtime 程式碼。** 接受後依 `docs/tasks/refactor-ai-capability.md` 的批次序執行：

1. REF.1 — `UnifiedResult` schema（Rust + TS + shim）。
2. REF.2 — `CommandPalette.tsx` 拆分為 feature 模組。
3. REF.3 — `handlers/agent/mod.rs` 拆分。
4. REF.4 — `core/ai_capability/` + 初版三項 capability + IPC handler + 前端 hooks。
5. REF.5 — workflow memory（可在 REF.4 之後與其並行）。
6. REF.6 — `CommandPalette` 消費 `UnifiedResult`、移除嵌入式 `AiPanel`/`TerminalPanel` 熱路徑掛點。
7. REF.7 — 量化 gates、`ai.legacy_agent = false`、觀察一個 release cycle。
8. REF.8 — 物理移除 chat-first `AiPanel`、收斂或刪除 `agent_runtime.rs`、移除旗標。

## 7. Rollback Plan（回滾策略）

- **Feature flag 開關：** `ai.legacy_agent = true` 立即恢復 chat-first 行為與舊 ReAct loop。
- **REF.7 預設切換延後：** 若觀察期內出現 P0 回歸，延後預設切換到下一個觀察窗口；旗標仍可由使用者手動切換。
- **資料格式回滾：** workflow_history（REF.5）為加性 migration；rollback 指南附在 REF.5 交付物內，必要時可丟棄該表而不影響其他資料。
- **殘留檔案清理：** REF.8 之前任何中止都不刪除 chat/agent code，僅以旗標停用。
- **驗證退回：** chat-first 流程的既有手動回歸 + ReAct 整合測試重跑通過。

## 8. Validation Plan（驗證方式）

本 ADR 本身為文件草擬，主要驗證為 docs guard。詳細實作驗證引用 `docs/tasks/refactor-ai-capability.md` 的 Validation Matrix：

> **Amendment 2026-06-06（developer-approved，in-conversation）：** inline latency
> target 改為 **tiered**，並把參考/建議 default 模型由 `qwen2.5:7b` 改為
> `qwen2.5:1.5b`。理由：REF.7.D 量測證實瓶頸是 CPU token 吞吐，換模型無法單獨達原
> `<800ms` 目標（見 §10）。ADR 狀態維持 `accepted`（僅細化驗證目標，未變更 §4 Decision）。

| 測試類型    | 覆蓋目標                                                         | 指令 / 步驟                       |
| ----------- | ---------------------------------------------------------------- | --------------------------------- |
| Docs guard  | frontmatter / size / narrative                                   | `npm run docs:refresh`            |
| Unit        | 各 capability 純函式                                             | `cargo test ai_capability`        |
| Integration | Ollama `qwen2.5:1.5b` 端到端（參考 default;`qwen2.5:7b` 為高品質選項） | 本機 Ollama + integration test |
| Performance | inline **GPU/ideal tier** P50 < 800ms、P95 < 1500ms（aspirational） | REF.7 bench scripts            |
| Performance | inline **CPU-host tier** P50 < 5000ms、P95 < 8000ms（`qwen2.5:1.5b` realistic） | REF.7 bench scripts |
| Performance | palette cold < 200ms、warm < 50ms、search first chunk P50 < 80ms | REF.7 bench scripts               |
| Memory      | idle RSS 10min < 150MB、1h < 200MB                               | 手動量測 Task Manager             |
| Regression  | Bug A launcher focus/IME、Bug B delete verification              | REF.2 後、REF.7 前各一次手動      |
| Security    | approval / audit 路徑沒有縮減                                    | ADR-0022 既有測試 + manual review |

## 9. Open Questions（未解問題）

開發者於 2026-05-20 拍板下列五題；正式變更已寫入 §4 Decision。

- [x] `ActionChip` 是否允許跨 capability 鏈接？→ **Capability 內嚴格 single-shot，UI 層允許 user-driven chaining。**
- [x] workflow memory（REF.5）的 `context_hash` 是否成為 capability 輸入的一級參數？→ **不為一級必填；設為 capability schema 之 optional 欄位，由叫用端按需填入。**
- [x] `ai.legacy_agent` 觀察窗口長度的定義？→ **一個 minor release tag 與 14 個日曆天兩者較長者，且觀察期內無 P0 回歸。**
- [x] approval ownership 轉向 UI 後，後端 risk tag 的最小集合與序列化格式是否需另寫 ADR？→ **是；另立 ADR-0030 _Backend Risk Tag Contract_。**
- [x] `agent_runtime.rs` 在 REF.8 完全刪除，或保留審計面 shrink 版？→ **先抽出 audit-only 邏輯到 `core/agent_audit.rs`，再完整刪除 `agent_runtime.rs`。**

- [x] `core/agent_audit.rs` 的最終命名與模組位置 → **採用 `core/agent_audit.rs`，作用域嚴格為「legacy agent approval audit 歷史紀錄查詢」**；新 capability 的 audit 不混入此模組（見 §4）。
- [x] `ActionChip` 在 UI 層 user-driven chaining 是否需要 telemetry / 防誤觸節流 → **不採用 time-based throttle 或 telemetry**；改用 state-based「in-flight 期間 chip disabled」（見 §4）。REF.7 量化 gate 若需使用率資料再另行評估 telemetry。

## 10. Measurement（REF.7 量化讀數）

REF.7.C 的資料填充區。本節僅記錄量測讀數，不變更 §4 Decision；ADR 狀態維持 `accepted`。

正式讀數由 REF.7.D（user-action）產生（default 參考模型已改為 `qwen2.5:1.5b`）：

```
ollama pull qwen2.5:1.5b
npm run bench:ai -- --runs 10 --model qwen2.5:1.5b
```

讀數於 2026-06-06 產生（本機 Windows，**CPU-only，無 GPU**）。inline P50/P95 為
`explain`+`fix_error`+`gen_command` 三個非串流 inline capability 的合併樣本（n=30，
每 capability 10 runs）。原始 JSON 與 per-capability 分佈見 `sessions/2026-06-06.md`。

評估依 §8 Amendment 2026-06-06 的 **tiered** target;default 參考模型已改為
`qwen2.5:1.5b`，故以其為主讀數，`qwen2.5:7b` 列為高品質選項對照。

| 指標                   | CPU-host tier | GPU/ideal tier | 讀數 `qwen2.5:1.5b` (default) | 讀數 `qwen2.5:7b` | 狀態（CPU tier）           |
| ---------------------- | ------------- | -------------- | ----------------------------- | ----------------- | -------------------------- |
| inline P50             | < 5000 ms     | < 800 ms       | **4266 ms**                   | 7598 ms           | **PASS**（1.5b）/ 7b FAIL  |
| inline P95             | < 8000 ms     | < 1500 ms      | **6269 ms**                   | 11654 ms          | **PASS**（1.5b）/ 7b FAIL  |
| palette cold open      | < 200 ms      | < 200 ms       | —（bench:ai 不產生）          | —                 | pending（需另立 harness）  |
| palette warm open      | < 50 ms       | < 50 ms        | ~2.5 ms（webview-side, 1.H）  | —                 | PASS\*                     |
| search first chunk P50 | < 80 ms       | < 80 ms        | —（bench:ai 不產生）          | —                 | pending（需另立 harness）  |

\* palette warm open 讀數來自 PRODUCT.1.H 的 `PerfBadge` dogfood，量的是 webview 端
（事件→input 對焦繪製），不含 OS 按鍵→webview 事件;為 lower bound。

### 小模型重測（2026-06-06，回應「改小模型 default」方向）

| 模型         | inline P50 | inline P95 | 可靠性          |
| ------------ | ---------- | ---------- | --------------- |
| qwen2.5:7b   | 7598 ms    | 11654 ms   | 穩定            |
| qwen2.5:1.5b | 4266 ms    | 6269 ms    | 穩定（10/10）   |
| qwen3:0.6b   | ~4–5 s     | —          | **不穩**（空回應，bench 中止） |

關鍵結論：參數量降 4.6×（7b→1.5b）只把延遲砍半，**最佳可靠小模型 qwen2.5:1.5b 仍
4.3s P50、超標約 5.3×**。瓶頸是 **CPU token 生成吞吐**，非模型大小;本機無任何合理
模型能達 800ms。`qwen3:0.6b` 因 reasoning token 吃掉 512 token 預算而間歇空回應，不適
合當 default。註：`model_manager.recommend_models` 已是 hardware-tiered，低階機本就推薦
`qwen2.5:1.5b`;`handlers/ai.rs` 的 `qwen2.5:7b` 僅為 recommend 空清單時的 fallback。

**Resolution（developer-approved 2026-06-06）：** inline 延遲 gate 在本機 CPU 大幅
未達原 `< 800 ms` 目標，且**換小模型無法單獨解決**（瓶頸為 CPU token 吞吐）。開發者
裁定採 **tiered target**（§8 Amendment：保留 GPU/ideal `<800ms` 為 aspirational，新增
CPU-host tier `P50<5000 / P95<8000ms`）並把參考 default 改為 `qwen2.5:1.5b`。據此重評：
**`qwen2.5:1.5b` 的 inline P50 4266 / P95 6269 ms 通過 CPU-host tier**;`qwen2.5:7b`
仍超 CPU tier（列為高品質選項）。此舉**不變更 §4 Decision**（ADR 維持 `accepted`，僅
細化 §8 驗證目標）。`PRODUCT.2` 的解凍 gate 條件（REF.7.D + 方向定案）至此已滿足;開發者已於
2026-06-06 明示**解凍 `PRODUCT.2`**（見 `active.md` / `product-roadmap.md`）。

觀察窗口項目（`ai.legacy_agent` 預設關閉一個 release cycle、idle RSS 10min < 150 MB / 1h < 200 MB）於 REF.8 開窗時記錄，維持 `pending observation`。
