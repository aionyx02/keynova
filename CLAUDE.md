---
type: agent_bootstrap
status: active
priority: p0
updated: 2026-05-13
context_policy: always_retrievable
owner: project
---

# CLAUDE.md

> Auto-loaded at session start. Detailed governance and ADR rules are in `docs/CLAUDE.md`.

## Session Start (Retrieval-First)

1. Read `docs/index.md` first (single source for doc routing).
2. Read `docs/memory/current.md` (current strategy and constraints).
3. Read `docs/tasks/active.md` (what to do now).
4. Read `docs/tasks/blocked.md` only when planning, approval, security, or risky actions are involved.
5. Retrieve additional documents by user intent. Never inject all docs into context at once.

## Session Close (Mandatory Sync)

Before final response or commit:

1. Update task state in one of:
   - `docs/tasks/active.md`
   - `docs/tasks/backlog.md`
   - `docs/tasks/blocked.md`
   - `docs/tasks/completed.md`
2. Update `docs/memory/current.md` (what changed, next step, risks).
3. If architecture/security/testing behavior changed, update:
   - `docs/architecture.md`
   - `docs/security.md`
   - `docs/testing.md`
4. If decision boundary changed, add/update ADR in `docs/adr/` and refresh `docs/decisions.md`.
5. Run `npm run docs:refresh` to prevent documentation drift.

## Project Overview

Keynova is a keyboard-first productivity launcher built with Tauri 2.x + React + Rust.
Primary goal: complete 90%+ developer workflows without mouse interaction.
Supported platforms: Windows 10+, Linux (X11/Wayland), macOS 11+.

## Git Workflow Rules

```
main  <- always releasable
  <- dev (integration)
      <- feature/<name> (work branches)
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

- `docs/index.md` - documentation router and retrieval policy
- `docs/project.md` - stable project facts
- `docs/tasks/active.md` - active work only
- `docs/memory/current.md` - short working memory
- `docs/CLAUDE.md` - ADR and governance rules
