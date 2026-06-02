# Backend Risk Tag Contract For UI-Owned Confirmation

**狀態：** 接受
**日期：** 2026-05-20
**決策者：** 開發者
**相關文件：**

- docs/adr/0029-ai-capability-layer.md
- docs/adr/0022-agent-approval-boundary.md
- docs/security.md
- docs/tasks/refactor-ai-capability.md

---

## 1. Context（技術背景）

ADR-0029 將 approval ownership 從後端轉向 UI：後端標註動作的風險特徵，UI 依此決定是否求證。本 ADR 定義該標註的最小資料契約。

初版只有 `explain`、`summarize`、`fix_error` 三項 capability，實際只需要兩條規則：

- inline AI（如 explain / summarize）不彈確認，直接顯示結果。
- destructive 動作（如 fix_error 改檔案）必須彈確認。

歷史經驗（ADR-0022 approval boundary 與 chat-first AiPanel 的 UX 衝突）顯示：把 risk schema 設計過早會在實作前就堆出沒人用的維度。本 ADR 採最小契約 + 加性擴充規則，把細粒度留到實際需要時再寫 ADR。

## 2. Constraints（系統限制與邊界）

- Schema 在 Rust（後端）與 TypeScript（前端）對稱可序列化（JSON）。
- 必須 forward-compatible：缺欄位 / 未知欄位 / 解析失敗 → fail-safe to confirm。
- 不擴大既有權限邊界。
- `reason` 字串僅進入 audit log，**不渲染到 UI**——避免 prompt 走私 / log injection 顯示面風險，但因不顯示，本契約不要求字串 whitelist。
- Capability 的 audit 行為由其 **註冊時的 metadata** 決定（每個 capability 一次），不放在每次呼叫的回傳中。
- Inline AI 路徑必須維持 800ms P50 / 1500ms P95 預算；本契約結構必須能在 Rust → JSON → TS 之間以 sub-ms 處理完。

## 3. Alternatives Considered（替代方案分析）

### 方案 A：完整 category × severity × 多個旗標的結構化 schema（不採用）

**優點：** 表達力強、未來擴充無痛。
**缺點：** 初版三項 capability 只用得到 2 格；過度設計使 capability 作者必填值不一致、UI 條件分支膨脹。
**風險：** 增加實作前後審查面、減少實際性能與 UX 預算。

### 方案 B：最小契約 `{requires_confirmation, reason}` + 加性擴充規則（採用）

**優點：**

- 只暴露 capability 真正需要的欄位。
- forward-compat 規則允許未來加 `category` / `severity` 而不破壞 v1 client。
- audit_required 不在每次回傳中，省記憶體與序列化負擔。

**缺點：** 未來真的需要分級時要再寫一份 ADR。
**風險：** 加性擴充紀律弱化時 schema 仍可能膨脹 → 在 §6 明訂演進規則。

### 方案 C：不另立 ADR，欄位寫進 ADR-0029（不採用）

**優點：** 文件最少。
**缺點：** 未來改 schema 需要重審策略 ADR；違反 governance §7「Data format / public contract changes」要求。

## 4. Decision（最終決策）

選擇：**方案 B — 最小契約 `{requires_confirmation, reason}`**

JSON shape：

```json
{
  "requires_confirmation": false,
  "reason": "audit-only short string"
}
```

對應 Rust 與 TypeScript（示意，實作時調整 module 路徑）：

```rust
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct RiskTag {
    pub requires_confirmation: bool,
    #[serde(default)]
    pub reason: String,
}
```

```ts
interface RiskTag {
  requires_confirmation: boolean;
  reason?: string;
}
```

行為規則：

- **Fail-safe to confirm**：UI 收到缺 `risk_tag` 欄位、`risk_tag` 解析失敗、或 `requires_confirmation` 缺值 → 視同 `true`。
- **`reason` 僅進 audit log**：UI 不渲染、不顯示給使用者；不需要 i18n / whitelist。
- **`reason` 長度上限 256 UTF-8 bytes**：超過時由 **capability 端** truncate（Rust 以 `floor_char_boundary` 或 `char_indices()` 對齊 UTF-8 char boundary 後切，避免斷字元），末尾附 `…`；不允許 audit log 寫入端截斷（截斷意圖應發生在最了解語境的位置）。
- **Audit 政策由 capability metadata 決定**：每個 capability 在註冊時宣告 `audit: yes/no`，不在每次回傳中重複。
- **Capability 自行決定 `requires_confirmation`**：依當下動作分流。例：
  - `explain` / `summarize` 永遠 `false`。
  - `fix_error` 純解釋型（只顯示說明）`false`；觸碰檔案 / 執行命令的修復路徑 `true`。
- **Forward-compatibility**：未來新增欄位（例如 `category`、`severity`）為加性變更；舊 client 收到不認得的欄位忽略；移除 / 重新命名既有欄位需新 ADR。

Feature flag：N/A。

Migration 需求：與 REF.4 一同落地；既有 chat-first 路徑不消費此契約。

Rollback 需求：見 §7。

## 5. Consequences（系統影響與副作用）

### 正面影響

- Capability 作者不需要在每次回傳中做風險分類決策，認知負擔最小。
- UI 確認邏輯極簡（一個 bool 判斷），不在 inline AI 熱路徑增加可量測 overhead。
- 加性演進規則為未來細粒度留路，不阻止後續擴充。

### 負面影響 / 技術債

- 後續若 UI 需要客製化確認文案 / 多種風險視覺，需新 ADR 擴充欄位。
- `reason` 是 free-form 字串；維護紀律要求作者寫人類可讀但不含使用者輸入的內容。

### 對使用者的影響

- `explain` / `summarize` 不彈窗，鍵盤節奏不打斷。
- 動到檔案 / 命令的 `fix_error` 路徑會彈確認。

### 對開發者的影響

- 新增 capability 必須宣告 `audit` metadata，並決定 `requires_confirmation` 策略。
- TS / Rust 兩側型別小到不需 codegen，手動同步即可。

### 對測試的影響

- Schema round-trip 測試（unit）。
- Fail-safe 路徑測試（缺欄位 / 解析失敗 → confirm）。

### 對安全性的影響

- `reason` 不顯示 → 無 log injection / prompt 走私顯示面風險；audit log 端仍應對 reason 做長度上限。
- ADR-0022 audit 邊界透過 capability metadata 維持。
- ADR-0027 generic shell sandbox 限制不變。

## 6. Implementation Plan（實作計畫）

> 本 ADR 未被接受前不得修改 runtime 程式碼。接受後與 REF.4 同步交付。

1. `src-tauri/src/models/risk_tag.rs`：定義 `RiskTag` 結構與 serde。
2. `src/types/`：TypeScript interface + 解析 helper（含 fail-safe）。
3. REF.4 三項 capability 各自決定 `requires_confirmation`，並在註冊時宣告 audit metadata。
4. UI confirmation component：依 `requires_confirmation` 彈窗；fail-safe path 有單元測試。
5. Schema 演進規則在接受後同步寫入 `docs/architecture.md` 一節。

## 7. Rollback Plan（回滾策略）

- Feature flag：N/A。
- 若需回退：capability 不回傳 `risk_tag` → UI fail-safe 視同 confirm，所有動作都彈窗。降級為保守可用狀態，無資料毀損。
- 完整移除：刪除 `risk_tag.rs`、前端對應型別、UI 消費點。chat-first legacy 路徑無需 rollback。

## 8. Validation Plan（驗證方式）

| 測試類型    | 覆蓋目標                                | 指令                       |
| ----------- | --------------------------------------- | -------------------------- |
| Unit        | Rust serde round-trip（含缺欄位）       | `cargo test risk_tag`      |
| Unit        | TS 解析缺欄位 / 未知欄位 → fail-safe    | `npm run test -- risk-tag` |
| Unit        | `reason` 不入 UI 渲染路徑               | UI 測試                    |
| Integration | capability call 回傳 risk tag 並驅動 UI | REF.4 整合測試             |
| Security    | `reason` 不洩使用者輸入                 | code review                |

## 9. Open Questions（未解問題）

開發者於 2026-05-20 拍板下列兩題；正式變更已寫入 §4 行為規則。

- [x] `reason` 字串長度上限 → **256 UTF-8 bytes**；capability 端 truncate at char boundary，末尾附 `…`（見 §4）。
- [x] 未來真的需要 category × severity 時走新 ADR 還是修訂本 ADR？→ **新 ADR**（建議 `ADR-0040+ Risk Tag Contract v2`），保留 ADR-0030 為 v1 minimal contract 的歷史錨點；不修訂本 ADR 以避免標題與內容失配。
