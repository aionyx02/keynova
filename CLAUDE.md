# CLAUDE.md

Keynova is a keyboard-first launcher (Tauri 2 + React + Rust). The goal is
completing developer workflows without the mouse. AI is an inline capability,
not the product core. Windows 10+, Linux (X11/Wayland), macOS 11+.

## Documentation

Four files, and you read them when you need them — there is no startup ritual
and no routing layer:

- `docs/state.md` — where the project is, what is unfinished, what is true
  outside the repo
- `docs/decisions.md` — why things are the way they are, and the traps
- `docs/architecture.md` — the shape you cannot get one file at a time
- `docs/security.md` — what is refused and why

**Do not update documentation by default.** No session logs, no task queue, no
"update the smallest matching doc before replying". Those rules produced 118
files and 3267 lines of tooling to manage them, and the whole apparatus went
unused for ten weeks without anyone missing it.

Write something down only when you have produced a fact that **cannot be
recovered from the code or from git log**. There are exactly four kinds:

1. why a choice was made, including what was rejected
2. a trap — something that has bitten and will bite again
3. what is unfinished, or deliberately not being done
4. state outside the repo

Everything else — module layout, IPC routes, event topics, config shapes, what
changed in a commit — is already recorded somewhere that stays correct. Adding
a Markdown copy makes it wrong later, not clearer now. When you do write, say so
in your reply.

## Git

```text
main    <- always releasable
dev     <- integration
feature/<name>, chore/<name>  <- work branches, cut from dev
```

- **Never merge without explicit confirmation.** Show the diff first.
- `main` → `dev` sync is allowed and expected. The old rule forbidding it is
  what killed `dev` once: work forked from `main`, `dev` had no legal way to
  catch up, and it fell 191 commits behind.
- `main` and `dev` are protected. A PR needs 8 green checks; no approving
  review is required, because GitHub will not let a solo maintainer approve
  their own PR. The human pressing merge is the review.

## Commands

```bash
npm install && npm run tauri dev
npm run tauri build
npm run lint && npm run test
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
npm run verify          # everything, before a release
```

There are no git hooks. Checks run in CI.

## Boundaries

Read `docs/security.md` before touching anything on this list — several changes
require an ADR *before* implementation, not after.

Two rules that apply to you specifically:

- **You may not widen your own permissions.** Do not edit `.claude/`,
  `capabilities/default.json`, or the agent-configuration section of
  `docs/security.md`.
- **A PR touching this file or any agent configuration is a permission
  change**, and is reviewed line by line. "It's only documentation" does not
  apply here.
