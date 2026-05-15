---
type: decision_index
status: active
priority: p1
updated: 2026-05-15
context_policy: retrieve_only
owner: project
---

# docs/decisions.md — ADR Index

> Retrieval policy: 先由 `docs/index.md` 判斷是否需要 ADR，再精準讀取對應單一 ADR 檔案。

> 所有 ADR 已拆分為獨立檔案，存放於 `adr/`。
> 本文件僅作為快速索引。完整內容請閱讀對應的 ADR 檔案。

---

## ADR 流程快速參考

- ADR 格式：`adr/NNNN-kebab-title.md`（9 節標準格式）
- 完整模板：`adr/0000-template.md`
- 治理規則：`CLAUDE.md`
- **AI agent 不得自行將 ADR 狀態改為「接受」**

### 狀態定義

| 狀態 | 意義 |
|------|------|
| 提議 | 尚未允許實作 |
| 接受 | 開發者已審閱，可開始實作 |
| 拒絕 | 此方案被拒絕 |
| 廢棄 | 曾接受但已不再適用 |
| 取代 | 被新 ADR 取代 |

---

## ADR 清單

| 編號 | 標題 | 狀態 | 日期 |
|------|------|------|------|
| [0001](adr/0001-frontend-framework.md) | 前端框架選擇（React 18 + TypeScript） | 接受 | 2026-05-01 |
| [0002](adr/0002-state-management.md) | 前端狀態管理（Zustand） | 接受 | 2026-05-01 |
| [0003](adr/0003-styling.md) | 樣式方案（Tailwind CSS） | 接受 | 2026-05-01 |
| [0004](adr/0004-backend-framework.md) | 後端與桌面框架（Rust + Tauri 2.x） | 接受 | 2026-05-01 |
| [0005](adr/0005-terminal.md) | 終端方案（xterm.js + portable-pty） | 接受 | 2026-05-01 |
| [0006](adr/0006-dual-search-backend.md) | 文件搜尋雙後端（Tantivy + Everything） | 接受 | 2026-05-02 |
| [0007](adr/0007-development-ide.md) | 開發 IDE（JetBrains RustRover） | 接受 | 2026-05-01 |
| [0008](adr/0008-terminal-prewarm.md) | Terminal PTY Pre-warm 策略 | 接受 | 2026-05-03 |
| [0009](adr/0009-builtin-command-registry.md) | BuiltinCommand 可擴展指令 Registry | 接受 | 2026-05-02 |
| [0010](adr/0010-config-manager-toml.md) | ConfigManager TOML I/O | 接受 | 2026-05-02 |
| [0011](adr/0011-ai-provider-abstraction.md) | AI Provider 抽象化（AiProvider trait） | 接受 | 2026-05-03 |
| [0012](adr/0012-note-sync-strategy.md) | 筆記同步策略（Markdown 檔案） | 接受 | 2026-05-03 |
| [0013](adr/0013-system-control-permissions.md) | System Control 權限範圍 | 接受 | 2026-05-03 |
| [0014](adr/0014-action-knowledge-plugin-boundary.md) | Action / Knowledge / Plugin 邊界 | 接受 | 2026-05-05 |
| [0015](adr/0015-search-backend-preference.md) | Search Backend Preference 與重建入口 | 接受 | 2026-05-05 |
| [0016](adr/0016-agent-read-only-tools.md) | Agent Read-only Tool Runtime | 接受 | 2026-05-05 |
| [0017](adr/0017-automation-executor.md) | Automation Action Chain Executor | 接受 | 2026-05-06 |
| [0018](adr/0018-app-module-split.md) | App Module Split（app/ 子模組） | 接受 | 2026-05-06 |
| [0019](adr/0019-progressive-search-runtime.md) | Progressive Search Runtime（stream=true） | 接受 | 2026-05-06 |
| [0020](adr/0020-terminal-launch-specs.md) | Terminal Launch Specs for Builtin Commands | 接受 | 2026-05-06 |
| [0021](adr/0021-persisted-search-index.md) | Persisted Search Index And Runtime Ranking | 接受 | 2026-05-07 |
| [0022](adr/0022-agent-approval-boundary.md) | Agent Approval Boundary And Prompt Audit | 接受 | 2026-05-07 |
| [0023](adr/0023-react-tool-schema-foundation.md) | ReAct Tool Schema And Observation Safety Foundation | 接受 | 2026-05-08 |
| [0024](adr/0024-agent-system-indexer.md) | Agent SystemIndexer Search Path | 接受 | 2026-05-08 |
| [0025](adr/0025-web-search-providers.md) | Structured Agent Web Search Providers | 接受 | 2026-05-08 |
| [0026](adr/0026-provider-driven-react-loop.md) | Provider-Driven ReAct Agent Loop | 接受 | 2026-05-08 |
| [0027](adr/0027-generic-shell-sandbox.md) | Generic Shell Tool Sandbox Requirement | 接受（research，product 未解封）| 2026-05-09 |
| [0028](adr/0028-learning-material-review-local-context.md) | Learning Material Review Local Context Boundary | 提議 | 2026-05-15 |

---

## 待補強事項

- Everything IPC 完整實作（`everything_ipc.rs` 目前為佔位符）— 見 ADR-006
- ADR-027 product 解封：各平台剩餘工作（AppContainer / seccomp / App Sandbox entitlement）
- TD.2 / TD.3 / TD.4 架構演進屆時需新增對應 ADR（見 tasks.md）
