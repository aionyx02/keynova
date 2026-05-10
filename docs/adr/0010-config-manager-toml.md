# ConfigManager TOML I/O

**狀態：** 接受  
**日期：** 2026-05-02  
**決策者：** 開發者  
**相關文件：**
- docs/architecture.md
- docs/security.md

---

## 1. Context（技術背景）

需要一個可讀寫、可 runtime reload 的 config 管理，支援分 section 的 TOML 格式。使用者設定包含 hotkeys、terminal、launcher、search、AI provider key 等。

## 2. Constraints（系統限制與邊界）

- Config 存放於 OS 標準目錄（`keynova_config_dir()`），不硬編碼 APPDATA
- 使用者設定優先，缺少時 fallback 至 `default_config.toml`（bundle 內）
- Config reload 需不重啟 App 即可生效（熱重載）
- API key 等敏感設定需要遮蔽顯示

## 3. Alternatives Considered（替代方案分析）

### 方案 A：Flat HashMap<String, String>（TOML 攤平）

**優點：**
- `setting.get/set` IPC 介面簡單，鍵名可直接作為 UI label
- Flat HashMap 易於 diff（找出哪些 key 改變）

**缺點：**
- Bool/int/float 靠字串猜測型別（TD.3.C 計畫改善）

### 方案 B：Typed AppConfig struct

**優點：**
- 型別安全，無字串猜測
- 編譯期驗證設定結構

**缺點：**
- 每次新增設定欄位需修改 struct + serde 定義
- 動態設定（前端不知道 schema）較難處理

## 4. Decision（最終決策）

選擇：**方案 A — Flat HashMap<String, String>**

原因：
- IPC 介面簡單，前端可直接使用 key 名稱
- Diff 計算容易，熱重載實作簡單

犧牲：
- Bool/int/float 靠字串猜測型別

Feature flag：N/A

Migration 需求：N/A（新專案）

Rollback 需求：N/A

## 5. Consequences（系統影響與副作用）

### 正面影響
- Flat HashMap 使 `setting.get/set` IPC 介面極簡
- runtime reload 實作：`reload_from_disk()` + `diff()` + `apply_config_changes()`

### 負面影響 / 技術債
- Bool/int/float 靠字串猜測型別（TD.3.C 計畫改善為 typed AppConfig struct）
- 前端 `SettingPanel` 四 tab 呈現，onBlur 才呼叫 `setting.set`（非 onChange）

### 對安全性的影響
- API key 存於 ConfigManager（`ai.api_key`），明文存放於 config.toml
- SettingPanel 敏感設定需遮蔽顯示（TD.5 安全強化計畫）

## 6. Implementation Plan（實作計畫）

已實作完成（Phase 2）。TD.3.C typed struct 為後續計畫。

## 7. Rollback Plan（回滾策略）

Flat HashMap 向後相容，升級 typed struct 時需提供 migration helper。

## 8. Validation Plan（驗證方式）

| 測試類型 | 覆蓋目標 | 指令 |
|---------|---------|------|
| Unit test | TOML 讀取 / diff / reload | `cargo test -- config_manager` |
| Integration test | setting.get / set IPC | `cargo test -- dispatch` |

## 9. Open Questions（未解問題）

- [ ] TD.3.C：`AppConfig` typed struct 計畫，移除 bool/int 靠字串猜測的問題