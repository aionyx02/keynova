# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Session 開場協議（每次新對話必須執行）

每次開啟新 session，**必須先執行以下三步，再做任何事**：

1. 讀取 `memory.md` — 確認目前進度、上次完成事項、下一步
2. 讀取 `tasks.md` — 確認當前未完成任務與阻塞項目
3. 用一句話告訴使用者：「目前進度：[進度]，上次完成：[事項]，今天預計繼續：[下一步]，是否確認？」
4. 請讀取 'skill.md' 以了解你的能力範圍和限制。

取得使用者確認後才開始工作。

## Session 收尾協議（每次完成工作後必須執行）

完成任何任務後，**在回覆使用者之前**，必須自動更新以下文件：

### `tasks.md`
- 將剛完成的項目打 `[x]`
- 若產生新的子任務，補充至對應 Phase

### `memory.md`
- 更新「上次完成」為剛完成的事項
- 更新「下一步」為下一個待辦
- 在「Session 交接紀錄（最近 5 筆）」表格新增一行（格式：`| 今日日期 | 完成事項 | 遺留問題或注意事項 |`）
- **壓縮檢查**：若「Session 交接紀錄」筆數 ≥ 6，將最舊的若干筆合併摘要至「歷史摘要（已壓縮）」區塊，交接紀錄只保留最新 5 筆

### `decisions.md`（有新決策時才更新）
- 若本次工作中確認了新的技術選擇或架構決策，補充對應 ADR
- 若既有 ADR 內容有變動（superseded / rejected），更新其狀態

### 'skill.md'（有新技能或限制時才更新）
- 若本次工作中確認了新的技能或限制，補充對應說明

> 四個文件更新完畢後，才向使用者報告結果。

## Project Overview

**Keynova**（全鍵控制系統）is a keyboard-centric productivity launcher for developers built with Tauri 2.x + React 18 + Rust. It enables 90%+ of workflow operations without a mouse. The project is currently in the planning/documentation phase — the `files/` directory contains complete architecture specs and design documents, but source code has not been implemented yet.

Target platforms: Windows 10+, Linux (X11/Wayland), macOS 11+. Package size target: ~50MB.

## Git 工作流程（強制遵守）

### 分支策略

```
main       ← 永遠是可發布的穩定版本
  └─ dev   ← 整合分支，功能在此驗證
       └─ feature/<name>   ← 每個功能的獨立分支
```

### 開發新功能的步驟

```bash
# 1. 從 dev 建立 feature 分支
git checkout dev
git pull origin dev
git checkout -b feature/<功能名稱>

# 2. 開發完成後，推送 feature 分支
git push origin feature/<功能名稱>

# 3. 合併到 dev（等待使用者審查後才執行）
git checkout dev
git merge --no-ff feature/<功能名稱>

# 4. 在 dev 上再次確認無誤後，合併到 main（等待使用者審查後才執行）
git checkout main
git merge --no-ff dev
```

### AI 行為規則（不得違反）

1. **絕不自行執行合併**：merge 到 `dev` 或 `main` 之前，必須停下來，列出所有變更，等待使用者明確說「確認合併」
2. **逐行展示**：每次寫完程式碼，必須用 diff 格式列出全部變更，讓使用者逐行審查
3. **分支來源**：所有 feature 分支必須從 `dev` 建立，禁止從 `main` 直接開分支
4. **不跳過審查**：即使程式碼看起來沒問題，也不能省略審查步驟，必須等使用者親眼確認每一行
5. **合併前檢查清單**：合併前必須確認 lint 通過、tests 通過、使用者已審查 diff
6. **階段性 commit**：每完成一個可運作的里程碑（例如一個功能的骨架、一個驗收標準通過），必須立即執行 commit，不等到所有功能都完成才一次提交
7. **禁止 merge main 回 dev**：私人文件（tasks.md/memory.md/CLAUDE.md 等）在 main 上已被 .gitignore 排除，將 main merge 進 dev 會刪除本地工作檔案。

## Development Environment

- **IDE**：JetBrains RustRover（主要）
- **OS**：Windows 11
- **Rust toolchain**：stable（透過 `rustup`）
- **Node**：透過 `nvm` 或系統安裝

RustRover 的 `.idea/` 目錄已列入 `.gitignore`，不提交任何 IDE 設定。

## Build Commands

```bash
npm install           # Install Node dependencies
npm run tauri dev     # Dev mode with hot reload
npm run tauri build   # Production build
npm run test          # React unit tests
npm run lint          # ESLint

cargo build           # Rust backend
cargo test            # Rust unit tests
cargo clippy          # Rust linting
```

## Architecture

The system follows a 6-layer architecture:

1. **Presentation** — React + Tauri frontend (command palette, terminal, floating windows)
2. **IPC Bridge** — Tauri + EventBus (RPC routing, real-time event push)
3. **Core Framework** — `CommandRouter` / `EventBus` / `ConfigManager` / `SearchRegistry`
4. **Business Logic** — Handlers → Managers (launcher, hotkey, terminal, mouse, search, system)
5. **Indexer** — Pluggable search (FileIndexer via tantivy/Everything, AppIndexer, HistoryIndexer)
6. **Platform** — Conditional compilation for Windows/Linux/macOS

### Extensibility Pattern

New features are added by implementing Handler traits and registering with `CommandRouter` — the core is not modified. Four extensibility mechanisms:
- `CommandRouter` — all features are commands (trait-based)
- `EventBus` — backend-to-frontend push via broadcast channel
- `ConfigManager` — layered TOML config with runtime reload
- `SearchRegistry` — pluggable indexers for unified search

### Planned Source Layout

```
src-tauri/src/
├── core/        # CommandRouter, EventBus, ConfigManager, SearchRegistry
├── handlers/    # Feature implementations (launcher, terminal, hotkey, etc.)
├── managers/    # Internal business logic
├── models/      # Data structures
├── platform/    # Platform-specific code
└── utils/

src/
├── components/  # React UI components
├── hooks/       # Custom React hooks
├── stores/      # Zustand state management
├── services/    # IPC/backend API clients
└── types/       # TypeScript definitions
```

## Documentation

All design specifications live in `files/`:
- `全鍵控制系統_程序代碼架構_v2.md` — complete code architecture, 40+ RPC interfaces, directory structure
- `Keynova_項目計劃書_v2.0.md` — development timeline, KPIs, risk assessment
- `README.md` — features, hotkeys, architecture overview
- `QUICKSTART.md` — installation and usage
- `文檔索引.md` — documentation index with role-based navigation

## User Configuration (runtime)

```toml
# Windows: C:\Users\<User>\AppData\Roaming\Keynova\config.toml
# Linux/macOS: ~/.config/keynova/config.toml

[hotkeys]
app_launcher = "Ctrl+K"
mouse_control = "Ctrl+Alt+M"

[terminal]
font_size = 13
scrollback_lines = 1000

[launcher]
max_results = 10

[mouse_control]
step_size = 15
```