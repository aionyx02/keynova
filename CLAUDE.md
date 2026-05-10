# CLAUDE.md

> Auto-loaded by Claude Code at session start. Detailed AI governance rules → `docs/CLAUDE.md`.

## Session 開場協議（每次新對話必須執行）

1. 讀取 `docs/memory.md` — 確認目前進度、上次完成事項、下一步
2. 讀取 `docs/tasks.md` — 確認當前未完成任務與阻塞項目
3. 告知使用者：「目前進度：[進度]，上次完成：[事項]，今天預計繼續：[下一步]，是否確認？」
4. 讀取 `docs/CLAUDE.md` — AI agent 開發守則與 ADR 流程（23 節）

取得使用者確認後才開始工作。

## Session 收尾協議（每次完成工作後必須執行）

在回覆使用者之前，**依序更新**：

1. `docs/tasks.md` — 完成項打 `[x]`，新子任務補充至對應 Phase
2. `docs/memory.md` — 更新「上次完成」與「下一步」，Session 交接紀錄新增一行；筆數 ≥ 6 時壓縮舊紀錄
3. `docs/adr/` — 有新架構決策時建立 `NNNN-title.md`，更新 `docs/decisions.md` index
4. 詳細文件同步規則見 `docs/CLAUDE.md` §17

## Project Overview

**Keynova**（全鍵控制系統）是以鍵盤為核心的生產力啟動器，採用 Tauri 2.x + React 18 + Rust。目標是讓開發者 90%+ 的工作流程操作不需要滑鼠。平台：Windows 10+ / Linux (X11/Wayland) / macOS 11+。包體積目標：~50MB。

## Git 工作流程（強制遵守）

```
main  ← 永遠是可發布的穩定版本
  └─ dev   ← 整合分支
       └─ feature/<name>   ← 每個功能的獨立分支
```

### AI 行為規則（不得違反）

1. **絕不自行 merge**：merge 到 `dev` 或 `main` 前，必須列出所有變更，等待使用者明確說「確認合併」
2. **分支來源**：所有 feature 分支必須從 `dev` 建立，禁止直接從 `main` 開分支
3. **逐行審查**：寫完程式碼後必須用 diff 格式列出全部變更讓使用者確認
4. **合併前檢查**：lint 通過、tests 通過、使用者已審查 diff
5. **階段性 commit**：每完成一個可運作里程碑立即 commit
6. **禁止 merge main 回 dev**：私人文件在 main 已被 `.gitignore` 排除，merge 回 dev 會刪除本地工作檔案

### 開發步驟

```bash
git checkout dev && git pull origin dev
git checkout -b feature/<名稱>
# 開發 → commit → 等使用者確認後才 merge
```

## Build Commands

```bash
npm install && npm run tauri dev    # Dev（hot reload）
npm run tauri build                 # Production build
npm run lint                        # ESLint
cargo test && cargo clippy          # Rust 測試 + lint
```

## Documentation

| 文件 | 用途 |
|------|------|
| `docs/CLAUDE.md` | AI agent 開發守則（23 節，含 ADR 流程）|
| `docs/architecture.md` | 系統架構、IPC、EventBus、儲存設計 |
| `docs/decisions.md` | ADR 索引（指向 docs/adr/）|
| `docs/tasks.md` | 任務進度（唯一版本）|
| `docs/memory.md` | Session 記憶與歷史（唯一版本）|
| `docs/testing.md` | 測試策略與指令 |
| `docs/security.md` | 安全邊界與敏感資料規則 |
| `docs/adr/0001-0027.md` | 各 ADR 獨立決策文件（27 個）|