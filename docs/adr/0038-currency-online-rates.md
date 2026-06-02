# Currency Online Rates — Network Boundary

**狀態：** 提議
**日期：** 2026-05-18
**決策者：** 開發者
**相關文件：**

- docs/security.md (§5 網路存取)
- docs/architecture.md
- docs/tasks/backlog.md (UTIL.1.B)

---

## 1. Context（技術背景）

UTIL.1.B 已交付 offline 貨幣換算：CalculatorManager 內含 18 條 USD-base hard-coded rate snapshot，
結果末尾附 `(offline rate, snapshot YYYY-MM)` 提示讓使用者察覺 stale。

問題：

- Offline 表會老化，使用者期待 launcher 算出來的數字接近實際匯率。
- 直接接 online API 屬「新增對外網路連線目標」，依 docs/security.md §5.2 必須先 ADR。
- exchangerate.host / open.er-api.com 等公開匯率服務沒有 auth，但仍然是新的外網入口。

## 2. Constraints（系統限制與邊界）

- 任何網路呼叫必須通過 `security.network_allowlist` 設定檢查。
- TLS 強制（不得使用 `danger_accept_invalid_certs`）。
- Timeout 嚴格：HTTP 連線 2s、總請求 5s。
- **不得傳送任何使用者資料**：URL query 只含 `?base=USD`，無 query 包含使用者輸入的 amount 或 source/target code。Request body 為空（GET only）。
- 快取必須降低重複呼叫：24h TTL，記憶體 + 磁碟（`~/.keynova/cache/currency_rates.json`）。
- Offline fallback 永遠可用：網路失敗 / timeout / 設定關閉 → 回 offline 表 + `(offline rate, ...)` suffix。
- 使用者必須能單獨關閉 online lookup（`calculator.currency_online_enabled` 預設 `false`）。
- 不需要 API key — 若選的 provider 改成需付費或需 key，必須再寫 ADR。

## 3. Alternatives Considered（替代方案分析）

### 方案 A：永久維持 offline-only（不採用）

**優點：** 零網路曝露、零 ADR、零維護成本。
**缺點：** Rate 永遠 stale；對活躍貨幣（CNY/TWD/JPY）誤差容易 ±5%。
**風險：** 使用者依賴錯誤匯率做小型交易判斷。

### 方案 B：自架快取 proxy（不採用）

**優點：** 控制權完全在我們這邊。
**缺點：** 需要 server 基礎建設；本地應用不該綁伺服器。
**風險：** 違反 keynova 完全本地化原則。

### 方案 C：opt-in 公開 API + 本地快取 + offline fallback（採用）

**優點：**

- 使用者 opt-in、預設關閉、不影響 `agent.local_context.enabled = false` 之外的隱私邊界。
- 24h 快取 + 公開無 auth API 把單機網路曝露降到最小。
- Offline fallback 保證即使網路 / API 異常仍能算。

**缺點：**

- 新增 1 個外網目標 + 維護 cache 檔案。
- 公開 API 的可用性受第三方影響。

**風險：** 第三方下架 → 自動 fallback 至 offline；catch-all 不會 panic。

### Provider 選擇

| Provider          | URL                                             | Auth | 註                              |
| ----------------- | ----------------------------------------------- | ---- | ------------------------------- |
| exchangerate.host | `https://api.exchangerate.host/latest?base=USD` | 無   | 主選 — 完全 free，無 key        |
| open.er-api.com   | `https://open.er-api.com/v6/latest/USD`         | 無   | 備援，如 exchangerate.host 下架 |

只允許 GET、`base=USD` 固定，不可帶其他 query。

## 4. Decision（最終決策）

選擇：**方案 C — opt-in 公開 API + 本地快取 + offline fallback**

原因：

- 對齊 docs/security.md §5 已允許的 read-only 公開 endpoint 模式（GitHub releases / 翻譯 API）。
- `calculator.currency_online_enabled` 預設 `false` 確保最小曝露面，與 ADR-028 的 `agent.local_context.enabled` 同模式。
- Offline fallback 保證壞掉時退化，不會 break 既有 `100 USD to TWD` 行為。

Feature flag：`calculator.currency_online_enabled`（預設 `false`）+ 加入 `security.network_allowlist`。

Rollback：設定關閉 → 立即回 offline；完整移除：刪除 `currency_rate_fetcher.rs`、移除 cache 檔、還原 settings_schema。

## 5. Consequences（系統影響與副作用）

### 正面影響

- 使用者啟用後可獲得 24h 內最新匯率。
- 維持 offline-first：壞網路不影響計算機可用性。

### 負面影響 / 技術債

- 新檔案 `managers/currency_rate_fetcher.rs` + cache 檔案。
- `security.network_allowlist` 預設值要加 `api.exchangerate.host` 與 `open.er-api.com`。
- Setting 面板多 1 個 boolean 設定。

### 對安全性的影響

- **新增 2 個外網目標**：`api.exchangerate.host`、`open.er-api.com`。docs/security.md §5.1 需更新。
- **無使用者資料外流**：URL query 固定為 `?base=USD`；不傳 amount、不傳 source/target、不傳 IP 以外的 metadata。
- TLS 強制 + 2s/5s timeout + 24h cache → 攻擊面極小。

## 6. Implementation Plan（實作計畫）

> 待 ADR-038 狀態改為「接受」後才能進入實作。

1. UTIL.1.B-online.1: `settings_schema.rs` + `default_config.toml` 加 `calculator.currency_online_enabled = false`；`security.network_allowlist` default 加 `api.exchangerate.host,open.er-api.com`。
2. UTIL.1.B-online.2: `models/currency.rs` — `CurrencyRateSnapshot { base: "USD", rates: HashMap<String, f64>, fetched_at: SystemTime }`。
3. UTIL.1.B-online.3: `managers/currency_rate_fetcher.rs` — `fetch_with_cache()`：先讀 cache（24h TTL），過期 → reqwest GET、解 JSON、寫 cache、回傳。網路失敗回 offline table。
4. UTIL.1.B-online.4: `CalculatorManager::try_currency_conversion` 接 fetcher：online 啟用時用 cached rate；rate 過期且網路失敗 → fall back to offline + 加 hint。
5. UTIL.1.B-online.5: 測試：cache hit / miss、TTL 過期、網路 timeout、JSON 解析失敗、provider 改用備援。
6. UTIL.1.B-online.6: docs/security.md §5.1 加 row、docs/decisions.md 標 ADR-038 接受、blocked.md 移除阻擋。

## 7. Rollback Plan（回滾策略）

- 設定 `calculator.currency_online_enabled = false` → 立即回 offline 表。
- 完整移除：刪除 `currency_rate_fetcher.rs`、移除 cache 檔案、還原 `security.network_allowlist` 與 settings_schema。

## 8. Validation Plan（驗證方式）

| 測試類型    | 覆蓋目標                                               | 指令                               |
| ----------- | ------------------------------------------------------ | ---------------------------------- |
| Unit        | cache TTL hit/miss                                     | `cargo test currency_rate_fetcher` |
| Unit        | JSON parse 不正常 → fallback                           | `cargo test currency_rate_fetcher` |
| Unit        | provider switch 邏輯                                   | `cargo test currency_rate_fetcher` |
| Unit        | offline fallback 仍輸出 `(offline rate, snapshot ...)` | `cargo test calculator_manager`    |
| Integration | network_allowlist 阻擋未允許 host                      | 手動 + 測試                        |
| Manual      | Setting 面板顯示開關                                   | 手動                               |
| Manual      | 網路斷線時計算仍可用                                   | 手動                               |

## 9. Open Questions（未解問題）

- [ ] Cache 檔案格式：JSON or TOML？JSON 較貼近 API 回應，TOML 較貼近其他 keynova config。建議 JSON。
- [ ] 是否需要 ETag / If-Modified-Since 支援以節省 bandwidth？對 24h 快取 + 18 currency 而言成本小，可不做。
- [ ] 若 exchangerate.host 與 open.er-api 兩家都掛點，是否要再加第三家？可先觀察使用者反饋。
- [ ] cached rates 是否該與 offline table 一起更新到 git？建議不要 — 保持 cache 為 local-only。
