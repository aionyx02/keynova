# AI Agent 開發守則與 ADR 決策流程

適用於 `docs/claude.md`、`docs/adr/0000-template.md` 與專案工程治理文件。

**文件目的**：約束 AI agent 在專案中的開發行為。核心原則是：重大技術決策必須先以 ADR 記錄，經開發者審閱並接受後，才可以修改受影響的正式程式碼。

---

## 0. 專案文檔結構

```
docs/
  adr/
    0000-template.md
    0001-frontend-framework.md
    ...（各 ADR 獨立檔案）
  architecture.md
  CLAUDE.md
  decisions.md
  memory.md
  security.md
  tasks.md
  testing.md
```

| 文件 | 用途 |
|------|------|
| `docs/adr/` | 存放 Architecture Decision Record，每一個重大技術決策都應有獨立 Markdown 檔案 |
| `docs/adr/0000-template.md` | ADR 模板，所有 ADR 必須遵守此格式 |
| `docs/decisions.md` | ADR 索引（指向 docs/adr/），快速查閱所有決策狀態 |
| `docs/architecture.md` | 系統整體架構、模組邊界、資料流、IPC、I/O、儲存設計 |
| `docs/tasks.md` | 任務拆解、目前進度、待辦事項與阻塞狀態 |
| `docs/memory.md` | 長期上下文、已知限制、歷史決策摘要，不得記錄未確認假設為事實 |
| `docs/testing.md` | 測試策略、測試指令、測試覆蓋要求 |
| `docs/security.md` | 安全邊界、權限模型、敏感資料處理原則 |
| `docs/CLAUDE.md` | AI agent 的專案操作守則與流程規範（本文件） |

---

## 1. 核心原則

安全性 > 資料正確性 > 可回滾性 > 可測試性 > 效能 > 開發速度 > 程式碼簡潔。

- ADR 的目的不是阻塞所有小型修改，而是管理會影響長期架構、效能、安全、資料格式、核心依賴或 public contract 的決策。
- **AI agent 不得自行將 ADR 狀態改為「接受」**。只有開發者可以接受 ADR。
- 若不確定是否需要 ADR，應優先建立 ADR 草案，而不是直接實作受影響的正式功能。
- ADR 只阻塞該決策直接影響的正式實作範圍，不阻塞文件、分析、測試計畫與無關的小型修正。

---

## 2. 正式程式碼的定義

本文件中的「正式程式碼」指會被產品 runtime、核心 library、CLI、server、worker、前端主流程或使用者可見功能直接使用的程式碼。

| 通常視為正式程式碼 | 通常不視為正式程式碼 |
|-------------------|---------------------|
| production runtime 程式碼 | 文件檔案，例如 `docs/*.md` |
| 核心 library、CLI、server、worker | ADR 文件 |
| 前端主流程、使用者可見功能 | 測試檔案、fixture、mock、stub |
| 正式 build 或部署流程依賴的 script | pseudo-code、example code、benchmark 草案 |

**例外**：若測試、benchmark、script 會被正式 build、部署流程或安全流程依賴，則視為正式程式碼。

---

## 3. ADR 狀態與權限

| 狀態 | 意義 | AI agent 權限 |
|------|------|--------------|
| 提議 | AI 或開發者提出方案，尚未允許實作 | 可以建立 |
| 接受 | 開發者已審閱並接受，可以開始實作 | 不得自行設定 |
| 拒絕 | 此方案被拒絕，不得實作 | 不得實作該方案 |
| 廢棄 | 曾經接受或提議，但已不再適用 | 不得依此實作 |
| 取代 | 被新的 ADR 取代，必須標註取代者 | 必須遵守新的已接受 ADR |

- AI agent 只能建立狀態為「提議」的 ADR。
- AI agent 不得使用 Accepted、approved、已批准、我認為可接受等同義詞繞過審閱。
- 只有開發者明確修改 ADR 狀態為「接受」，或在對話中明確指示「接受 ADR 000X」，才可開始實作受該 ADR 影響的正式程式碼。

---

## 4. ADR 判斷邊界與例外

以下規則用於避免 AI agent 過度保守。小型、局部、可回滾、且不改變架構或安全邊界的修改，通常不需要 ADR。

| 通常不需要 ADR | 必須建立 ADR |
|--------------|------------|
| 修正 typo、註解、文件 | 會進入 production runtime 的核心依賴 |
| 補測試或修正測試 fixture | 改變 public API、CLI、資料格式、migration 或使用者可見行為 |
| 小型 bug fix，且不改變 public API、資料格式、安全邊界或核心演算法 | 擴大檔案、網路、權限或安全邊界 |
| 單一模組內部 private helper 的小型重構 | 改變模組間通訊、async runtime、IPC、WebSocket、queue、worker 或 channel 模型 |
| 在既有安全邊界內讀取固定路徑的專案檔案 | 改變核心路徑上的搜尋、索引、同步、cache、批次處理或大量資料演算法 |
| 小型固定資料集上的排序、過濾或轉換 | 引入難以回滾、難以測試或會造成長期維護成本的設計 |
| 僅用於開發、測試、lint、format 的非 runtime 依賴 | 新增使用者可控制路徑的讀寫、刪除、移動、遞迴掃描或同步 |

---

## 5. ADR 強制觸發條件

### 5.1 新增或替換核心依賴

- 引入新的核心套件、Crate、npm package、Python package、系統服務或框架。
- 替換既有的資料庫、搜尋引擎、序列化格式、ORM、狀態管理方案。
- 引入需要背景服務、daemon、native binary、外部 process 的依賴。
- 引入會影響 build size、runtime memory、啟動時間或部署流程的依賴。
- 引入會改變跨平台支援狀況的依賴。

| 依賴分級 | 判斷標準 |
|---------|---------|
| 核心依賴 | 會進入 production runtime；影響資料儲存、搜尋、網路、安全、序列化、並行模型或 UI rendering；引入 native binary、背景服務、系統權限或平台限制；未來移除成本高 |
| 輕量依賴 | 僅用於測試、lint、format、type check；僅用於 build-time code generation 且不進入 runtime；小版本升級且沒有 breaking change |

### 5.2 改變模組間通訊或 async/sync 模型

- 從 function call 改為 IPC，或從 IPC 改為其他通訊模式。
- 從 HTTP 改為 WebSocket，或引入 queue、event bus、pub/sub、actor model。
- 從單程序改為多程序，或引入 worker、background task、thread pool。
- 改變 public API 或模組邊界上的同步/非同步型態。
- 導入新的 async runtime，例如 Tokio、async-std。
- 可能造成 deadlock、race condition、backpressure、lifetime、ownership 或 cancellation model 改變。

**例外**：單一模組內部 private helper 的小型 async 調整，若不改變 public API、不引入新 runtime、不改變通訊模型，通常不需 ADR。

### 5.3 涉及核心演算法或複雜度變化

- 新增或改寫核心搜尋、排序、索引、同步、diff、cache、排程、路徑掃描、檔案分析演算法。
- 任何可能達到或超過 O(n log n)，且位於核心路徑、熱路徑、啟動流程、UI blocking path、檔案掃描流程、搜尋流程、同步流程，或預期資料量可能持續成長的演算法變更。
- 對大量檔案、大量節點、大量事件、大量文字資料進行處理。
- 需要建立索引、cache、batching、pagination、streaming、incremental update 的設計。

**例外**：若排序、搜尋或資料轉換只發生在小型固定資料集（少量設定項、少量 UI 選項、測試資料或明確上限低於 100 筆的集合），通常不需 ADR。

### 5.4 改變安全邊界或檔案 I/O 權限

- 新增使用者可控制路徑的讀取、寫入、刪除、移動或遞迴掃描。
- 擴大既有檔案存取範圍。
- 改變 allowlist、denylist、sandbox、capability 或 workspace root 的定義。
- 新增會處理使用者私人檔案、系統目錄、home directory、外部掛載磁碟或網路磁碟的功能。
- 新增大量檔案掃描、索引、同步、刪除或批次修改。
- 改變 symlink、canonicalization、path normalization 或 permission error 的處理方式。

**例外**：在既有安全邊界內讀取固定路徑的專案設定檔、修改測試 fixture、讀取 repository 內部文件或測試資料、修正既有 I/O 行為中的明確 bug 且不擴大權限範圍，通常不需 ADR。

### 5.5 改變資料模型、持久化格式或 public contract

- 新增或變更 database schema、JSON、TOML、YAML、SQLite、binary format。
- 改變 config file、cache、index 的持久化格式。
- 需要 migration、rollback、compatibility layer，或舊版本資料可能無法讀取。
- 修改 CLI command、flag、argument、exit code。
- 修改 public function signature、plugin API、設定檔格式、預設行為。
- 刪除或 rename 已存在功能，導致既有使用者腳本失效。

---

## 6. ADR 阻塞範圍與任務拆分

ADR 只阻塞該決策直接影響的正式實作範圍。

ADR 尚未接受前，AI agent 仍可以：
- 撰寫或更新 ADR
- 更新 `docs/tasks.md`
- 更新測試計畫
- 撰寫不會被 production 使用的 pseudo-code
- 補充風險分析
- 建立 benchmark 設計
- 分析既有程式碼

若一個任務同時包含需要 ADR 與不需要 ADR 的部分，AI agent 應拆分處理：
- 可以先修 typo、補文件、補現有測試、分析現有架構、建立 ADR
- 不得先實作受 ADR 影響的正式功能

回報時必須明確列出：
1. 已完成的安全子任務
2. 被 ADR 阻塞的子任務
3. 需要開發者審閱的決策

---

## 7. 「停止實作」的定義

本文件中的「停止」、「停下」或「不得直接實作」是指：
- 不得修改受該決策影響的正式程式碼
- 不得引入相關正式依賴
- 不得改變 runtime 行為
- 不得改變資料格式或安全邊界

但 AI agent 仍應完成以下動作：
1. 說明為何停止實作
2. 建立或更新 ADR
3. 在 ADR 中列出 Open Questions
4. 更新 `docs/tasks.md` 的阻塞狀態
5. 回報需要開發者決策的項目

---

## 8. 使用者要求跳過 ADR 時的處理

若使用者要求「先不要寫 ADR」、「直接實作」、「不用管流程」或類似指令，但該任務已觸發 ADR 條件，AI agent 仍不得直接實作。

標準回應：

> 此任務觸發 ADR 規則，不能跳過決策流程。我可以先建立 ADR，或將任務拆成不需要 ADR 的安全子任務先處理。

---

## 9. 文件衝突解決順序

若文件之間出現衝突，AI agent 必須依照以下優先順序判斷：

1. 開發者在當前對話中的明確指示
2. 已接受的 ADR（`docs/adr/`）
3. `docs/security.md`
4. `docs/architecture.md`
5. `docs/claude.md`（本文件）
6. `docs/testing.md`
7. `docs/tasks.md`
8. `docs/memory.md`

若衝突涉及安全性、資料正確性或不可逆變更，必須停止實作並提出 Open Questions。

---

## 10. AI Agent 標準工作流程

1. 閱讀 `docs/architecture.md`、`docs/tasks.md`、`docs/memory.md`、`docs/testing.md`、`docs/security.md`、`docs/claude.md` 與 `docs/adr/`。
2. 判斷任務是否觸發 ADR。
3. 若不觸發 ADR，進行最小範圍實作，並同步更新測試與文件。
4. 若觸發 ADR，建立具流水號的 ADR，狀態設為「提議」，並停止受影響的正式實作。
5. 等待開發者審閱並接受 ADR。
6. ADR 接受後，只實作 ADR 中明確接受的範圍。
7. 完成後回報修改內容、測試結果、文件更新與風險。

---

## 11. ADR 檔案命名規則

```
docs/adr/NNNN-short-kebab-case-title.md
```

- 編號必須遞增，固定四位數。
- 不得覆蓋既有 ADR。
- 標題使用英文 kebab-case。
- ADR 內容可以使用中文，但技術名詞應保持原文。
- 若不確定下一個編號，必須先檢查 `docs/adr/` 既有檔案。
- 若同一主題已有 ADR，應優先更新狀態或新增 supersede 關係，而不是重複建立相同決策。

---

## 12. ADR 標準模板（九節）

完整模板見 `docs/adr/0000-template.md`。摘要結構：

```markdown
# [決策標題]

**狀態：** 提議
**日期：** YYYY-MM-DD
**決策者：** 開發者 / AI agent / 團隊
**相關文件：**
- docs/architecture.md

## 1. Context（技術背景）
## 2. Constraints（系統限制與邊界）
## 3. Alternatives Considered（替代方案分析）
## 4. Decision（最終決策）
## 5. Consequences（系統影響與副作用）
## 6. Implementation Plan（實作計畫）
## 7. Rollback Plan（回滾策略）
## 8. Validation Plan（驗證方式）
## 9. Open Questions（未解問題）
```

---

## 13. 程式碼修改規則

- 每次修改前必須確認會改哪些檔案、是否影響 public API、測試、安全邊界、文件與 ADR。
- 優先採用小步修改：小範圍修正 → 補測試 → 引入抽象層 → 逐步替換 → 最後才刪除舊實作。
- **不得進行無關修改**，例如大量 rename、重新格式化整個專案、重排 unrelated imports、順手修無關 bug。
- 若發現無關問題，應記錄在 `docs/tasks.md`，而不是直接修改。
- 不得用猜測補齊關鍵設計。若使用者資料存放位置、安全權限、目標平台、最大資料量或 backward compatibility 不明，應寫入 ADR 的 Open Questions。

---

## 14. 測試規則

修 bug、新增功能、改變資料格式、錯誤處理、檔案 I/O、安全權限、核心演算法、public API 或 CLI 行為時，必須新增或更新測試。

不得為了滿足測試要求而撰寫沒有實質驗證價值的測試。

**低價值測試包括：**
- 只測函式存在
- 只測 mock 被呼叫但不驗證行為
- 只測 happy path 卻忽略真正影響的邊界條件
- 複製實作邏輯到測試中

測試應優先覆蓋：核心行為、regression case、錯誤處理、安全邊界、資料格式相容性、rollback 或 migration 行為。

如果新增或修改測試，必須同步更新 `docs/testing.md`，記錄測試指令、測試範圍、測試資料、慢速測試與平台限制。

| 測試類型 | 使用時機 |
|---------|---------|
| unit test | 純函式、資料轉換、演算法 |
| integration test | 多模組協作、I/O、資料庫、IPC |
| regression test | 修復已知 bug |
| security test | 權限、路徑、敏感資料 |
| performance test | 搜尋、索引、批次處理、大量資料 |
| migration test | 資料格式升級或降級 |

---

## 15. 安全規則

### 15.1 檔案路徑處理

- 不信任使用者輸入的 path。
- 必須處理 `..`、symlink、absolute path、Windows drive prefix、Unicode normalization。
- 必須避免 path traversal、掃描超出允許目錄、follow symlink 造成逃逸。
- 涉及使用者控制路徑時，必須分析 TOCTOU 與權限不足檔案。

### 15.2 敏感資料

- 不得將 token、API key、password、cookie、private key、credential、OAuth refresh token、session secret 寫入 log、error message、cache、snapshot 或測試 fixture。
- 不得記錄使用者完整家目錄或私人檔案路徑。
- 若需要顯示，必須遮蔽，例如 `sk-****` 或 `/Users/***/secret.txt`。

### 15.3 網路存取

- 若功能涉及網路，必須先建立 ADR。
- ADR 必須分析：連線目標、離線運作、timeout、retry、TLS 驗證、proxy、telemetry、使用者是否可關閉、是否會傳送本機檔案內容或 metadata。

---

## 16. 效能規則

- 涉及大量資料時，必須明確寫出資料規模假設，例如 `n = 檔案數量`、`m = 每個檔案平均大小`、`q = 查詢次數`、`d = 目錄深度`、`e = 事件數量`。
- 必須分析：單次操作成本、多次操作成本、最差情況、cache 命中與未命中差異、memory growth、I/O amplification。
- 以下操作不得直接在 UI thread 或 latency-sensitive path 執行：
  - 遞迴掃描大量檔案
  - 大檔案讀取
  - 全文索引建立
  - 壓縮與解壓縮
  - 大量 JSON parse/stringify
  - 大量排序
  - network request
  - database migration
  - 大檔案 cryptographic hashing
- 若不可避免，必須在 ADR 說明 progress、cancellation、資料損壞防護與重入問題。

---

## 17. 文件同步規則

| 修改內容 | 需要更新 |
|---------|---------|
| 架構變更 | `docs/architecture.md` |
| 任務狀態 | `docs/tasks.md` |
| 測試方式 | `docs/testing.md` |
| 安全模型 | `docs/security.md` |
| AI 操作規則 | `docs/claude.md` |
| 技術決策 | `docs/adr/NNNN-title.md` |
| 已知限制 | `docs/memory.md` |

---

## 18. docs/tasks.md 建議格式

```markdown
# Tasks

## In Progress
- [ ] 任務名稱
  - 狀態：
  - 相關 ADR：
  - 相關檔案：
  - 阻塞原因：

## Pending
- [ ] ...

## Done
- [x] ...

## Blocked
- [ ] ...
  - 阻塞原因：等待 ADR 000X 被接受。
  - 需要開發者決策：
```

---

## 19. docs/memory.md 記憶規則

AI agent 不得將推測、假設或未確認資訊寫入 `docs/memory.md` 的 Stable Facts。若資訊尚未確認，必須放入 Assumptions 或 Open Questions。不得寫入敏感資訊。

```markdown
# Project Memory

## Stable Facts
- 本專案使用 Rust 作為核心邏輯。

## Constraints
- 必須支援 Windows/macOS/Linux。

## Assumptions
- [待確認] ...

## Open Questions
- ...

## Historical Notes
- ADR 0001 決定使用 ...

## Known Risks
- 大量檔案掃描可能造成 I/O 壓力。
```

---

## 20. 完成修改後的回報格式

```markdown
## Summary
- 修改了什麼。
- 為什麼修改。
- 是否符合相關 ADR。

## Files Changed
- `path/to/file`
- `path/to/test`

## Tests
- [x] 已執行：`command`
- [ ] 未執行，原因：

## Docs Updated
- [x] `docs/architecture.md`
- [x] `docs/testing.md`
- [ ] 不需要，原因：

## Risks
- 可能風險：
- 回滾方式：
```

---

## 21. 禁止行為

- 在 ADR 未接受前實作受限正式功能。
- 自行把 ADR 狀態改成接受，或用同義詞繞過審閱。
- 引入依賴但不說明理由。
- 修改安全邊界但不更新 `docs/security.md`。
- 改變資料格式但不提供 migration 或 rollback。
- 寫入敏感資訊到 log 或測試。
- 大規模重構但沒有任務或 ADR。
- 為了讓測試通過而刪除測試。
- 靜默忽略 failing tests。
- 使用 mock 掩蓋真實整合問題。
- 編造測試結果或 benchmark 結果。
- 刪除使用者資料。
- 改變 public API 卻不記錄 breaking change。
- 將暫時 workaround 偽裝成正式設計。

---

## 22. ADR 建立後的標準回覆

```
此任務涉及架構/效能/安全層級決策，因此我沒有直接修改受影響的正式程式碼。

我已建立：
- `docs/adr/000X-title.md`

目前狀態：`提議`

需要你審閱以下內容：
- Decision 是否可接受。
- Consequences 是否能承擔。
- Rollback Plan 是否足夠。
- Validation Plan 是否完整。

請將 ADR 狀態改為「接受」後，我才能開始實作受該 ADR 影響的正式功能。
```

---

## 23. ADR 被拒絕或取代時的處理

- 若 ADR 被拒絕：AI agent 不得實作該方案，必須閱讀拒絕原因，必要時提出新 ADR，且不得重複提交同一方案。
- 若 ADR 被取代：
  - 舊 ADR 必須標示「狀態：取代」與取代者。
  - 新 ADR 必須標示取代哪份 ADR。
  - AI agent 實作時必須遵守最新且狀態為「接受」的 ADR。