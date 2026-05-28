---
type: agent_bootstrap
status: active
priority: p0
updated: 2026-05-20
context_policy: always_retrievable
owner: project
---

# CLAUDE.md

> Auto-loaded at session start. Detailed governance and ADR rules are in `docs/CLAUDE.md`.

## Session Start

1. Read `docs/index.md` for routing.
2. Read `docs/memory/current.md` for current strategy and constraints.
3. Read `docs/tasks/active.md` for the active queue.
4. Retrieve additional documents by intent. Never load all docs recursively.

## Session Close

Before final response or commit:

1. Update only the smallest matching state doc.
2. Put detailed execution notes, debugging narrative, and command-output history in `docs/memory/sessions/YYYY-MM-DD.md`.
3. Keep `current.md` and `active.md` as current-state indexes only.
4. Put completed-task detail in the session log and refresh the compact completed index when needed.
5. Run `npm run docs:refresh`.

## Project Overview

Keynova is a keyboard-first productivity launcher built with Tauri 2.x, React, and Rust.

Primary goal: complete 90%+ developer workflows without mouse interaction.

Supported platforms: Windows 10+, Linux (X11/Wayland), macOS 11+.

## Git Workflow Rules

```text
main <- always releasable
dev  <- integration
feature/<name> <- work branches
```

- Do not merge without explicit user confirmation.
- Create feature branches from `dev`.
- Show diffs and pass checks before merge.
- Never merge `main` back into `dev` for this repository policy.

## Build Commands

```bash
npm install && npm run tauri dev
npm run tauri build
npm run lint
cargo test && cargo clippy -- -D warnings
npm run docs:refresh
```

## Documentation Entry Points

- `docs/index.md` - documentation router
- `docs/project.md` - stable project facts
- `docs/memory/current.md` - short working memory
- `docs/tasks/active.md` - active work only
- `docs/CLAUDE.md` - governance and ADR rules
