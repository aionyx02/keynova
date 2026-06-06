---
type: task_plan
status: backlog
priority: p1
updated: 2026-06-06
context_policy: on_demand
owner: project
tags: [product-roadmap, workflow-dispatcher, technical-workers, roadmap]
---

# PRODUCT.ROADMAP - Technical Worker Workflow Dispatcher

Derived from the 2026-06-05 Keynova product planning document. This plan turns
the product direction into executable task batches while respecting the current
P0 freeze in `docs/tasks/active.md`.

Product position: **Keynova is a keyboard-first local workflow entry for
technical workers.** It is not a general launcher, not an AI chat app, and not a
catch-all productivity suite.

Execution gate: **freeze lifted (developer decision 2026-06-06).** `PRODUCT.0`
(positioning), `PRODUCT.1` search-core, and `PRODUCT.2` useful inline AI
capabilities are **complete**. `PRODUCT.3` trusted release is the next product
track. PRODUCT.2 keeps generated commands copy-only; risk tags are display
metadata, not execution gates.

## Product Principles

- Keyboard first: primary workflows must be complete without the mouse.
- Local first: local capabilities are the default; external services are opt-in.
- Search before chat: the palette is a dispatcher, not a chat entry point.
- Suggest before execute: AI and automation propose; action gates execute.
- Explicit confirmation for risk: destructive file actions and shell execution
  must cross a clear confirmation boundary.
- Workspace context over global noise: current workspace and recent work should
  outrank generic global matches.
- Small capabilities over autonomous agents: single-step capabilities ship
  before agentic chains.
- Fast path before feature richness: daily paths must feel stable and fast
  before optional surfaces expand.

## Target Workflows

Canonical loop:

```text
Search -> Understand -> Act -> Remember -> Continue
```

Target users:

- Computer science students: switch course projects, run homework commands,
  search files/notes, and fix compiler errors quickly.
- Software engineers: move across repos, terminals, files, docs, and recent
  tasks without leaving the keyboard.
- DevOps/SRE users: inspect services, ports, logs, Docker/Kubernetes commands,
  risk labels, and reusable command cards.
- Data/ML engineers: run scripts/notebooks, manage model/data commands, preview
  files, and use a local-model helper.
- Power users: replace mouse-heavy local workflow switching with keyboard-first
  utilities and action chips.

## PRODUCT.0 - Positioning Lock

Goal: make the product direction explicit before more feature work resumes.

Scope:

- [ ] Update README positioning to the short product statement:
  "Keyboard-first local workflow entry for technical workers."
- [ ] Record non-goals: general launcher parity, AI chat center, autonomous
  agent mainline, and broad productivity-suite expansion.
- [ ] Add a compact roadmap pointer so users can see what is core vs parked.
- [ ] Align product copy in docs with `search-first`, `local-first`, and
  `approval-aware` architecture principles.

Done:

- README describes the product in one screen without promising broad launcher or
  AI-agent behavior.
- Docs distinguish MVP/core workflow from optional or parked features.
- No runtime behavior changes.

Non-goals:

- No new feature implementation.
- No release marketing polish beyond accurate positioning.

## PRODUCT.1 - v0.6 Stable Workflow Core

Goal: make the launcher core stable enough for daily technical work. This is
the "workflow dispatcher" release.

Depends on:

- `REF.6` dispatcher work materially complete.
- `REF.7` quality gates reviewed so new product work does not hide refactor
  regressions.

### PRODUCT.1.A - Ranking Baseline

Scope:

- [ ] Implement or audit a transparent ranking score:
  `text_match + workspace_context + recency + frequency + action_success -
  risk_penalty - noise_penalty`.
- [ ] Prioritize current workspace files, README files, config files, common
  project directories, and recently successful actions.
- [ ] Add rank-reason metadata where the UI already has room to explain why a
  result is high.
- [ ] Capture enough local-only usage signals to tune ranking without cloud
  telemetry.

Done:

- Same-name files prefer the current workspace when context is clear.
- Top 3 results match the expected item in the core workspace scenarios at least
  80% during manual dogfood checks.
- Ranking remains deterministic enough for unit tests around score components.

### PRODUCT.1.B - Unified Result Contract Audit

Scope:

- [ ] Audit app/file/workspace/command/history/memory/capability result sources.
- [ ] Confirm every source maps into `UnifiedResult` with stable `id`, `title`,
  `subtitle`, `source`, `rank`, `preview`, `primaryAction`,
  `secondaryActions`, `risk`, and `context` semantics.
- [ ] Remove source-specific UI branches that duplicate result behavior without
  adding a real capability.
- [ ] Keep action labels and risk badges consistent across result kinds.

Done:

- Result rendering and action dispatch are contract-driven.
- Adding a new result source does not require bespoke palette layout code.
- Existing tests cover at least one result from each source kind.

### PRODUCT.1.C - Workspace-Aware Search

Scope:

- [ ] Ensure current workspace search covers files, README, config files, and
  common developer directories before global results.
- [ ] Tune file indexing to avoid noisy generated directories and large private
  folders by default.
- [ ] Surface workspace identity clearly when the same file or command exists in
  multiple projects.
- [ ] Add regression fixtures for duplicate filenames across workspaces.

Done:

- Querying a common project file returns the current workspace match before
  global or stale matches.
- Generated/noisy directories do not dominate the first page of results.
- Workspace switching updates search context without an app restart.

### PRODUCT.1.D - Project Command Discovery

Scope:

- [ ] Discover commands from `package.json`, `Cargo.toml`, `Makefile`,
  `justfile`, and `docker-compose.yml`.
- [ ] Normalize common intents: `dev`, `test`, `build`, `lint`, `format`,
  `check`, `run`, and `preview`.
- [ ] Show command cards with command text, cwd/workspace, source file, risk,
  and rationale.
- [ ] Keep low-risk command copy/send fast, while state-changing or destructive
  commands require confirmation.

Done:

- Typing common intents in this repo returns correct npm/cargo commands.
- A project with both npm and cargo commands shows both without hiding cwd.
- Command cards can be copied from the keyboard.

### PRODUCT.1.E - Terminal Workflow — DROPPED (2026-06-05)

Developer decision: command execution / terminal handoff is **not built** — it
crosses the approval/security boundary and is the risk the developer wants to
avoid. Project commands stay copy-only (1.D); the user pastes into their own
terminal. `copy command` is already covered by 1.D. Revisiting requires a fresh
proposed ADR (ref ADR-0027, ADR-0022).

~~Scope (dropped):~~

- ~~Support `open terminal here` / `send command to terminal`.~~
- ~~Preserve cwd/provenance on send; classify + gate risk.~~

Done:

- Low-risk commands can be copied or staged quickly.
- High-risk commands show a risk badge and require explicit confirmation.
- Terminal handoff never silently changes cwd away from the selected workspace.

### PRODUCT.1.F - File Actions And Preview Polish

Scope:

- [ ] Tighten file actions: open, reveal, preview, rename, move, delete.
- [ ] Require confirmation for destructive or path-changing actions.
- [ ] Verify actual filesystem state after rename, move, and delete.
- [ ] Keep preview useful for text/config files without blocking the search fast
  path.

Done:

- Destructive actions have zero known confirmation bypass paths.
- Rename/move/delete failures leave visible, actionable feedback.
- Preview does not freeze palette input on large or binary files.

### PRODUCT.1.G - Developer Utilities MVP

Scope:

- [x] Ship keyboard-first utilities for UUID, hash, base64, JSON format, JWT
  decode, regex quick-check, and kill-port.
- [x] Return immediate results inline where possible.
- [x] Let `Ctrl+C` / copy button copy the primary utility output.
- [x] Mark utilities with side effects, especially kill-port, as risk-gated.

Done:

- Utility queries do not require panel navigation for the common case.
- Output is copyable from the keyboard.
- Side-effect utilities cannot run without confirmation.

### PRODUCT.1.H - Keyboard And Performance Gate

Scope:

- [x] Add CI-level coverage for MVP zero-mouse search-result paths.
- [x] Keep focus, IME, Enter, Escape, arrow navigation, and secondary-menu
  behavior stable at hook-test level.
- [x] Add manual dogfood checklist entries for the top 10 daily workflows.
- [ ] Capture a real `Ctrl+K` to input-ready timing sample on the primary
  Windows target machine during release dogfood.

Done:

- Search-result `Enter`, `Shift+Enter`, `Tab`, `Ctrl+C`, and secondary-menu
  `Enter` paths are covered by Vitest.
- Capability `Enter`, IME, `next` list navigation, and Escape unwind behavior
  have focused regression coverage.
- Manual release pass must still record physical timing, first useful result
  rank, keyboard completeness, and indexing/preview/streaming jank notes.

## PRODUCT.2 - v0.7 AI Capabilities That Are Useful

Goal: AI remains an inline helper for technical workflows: understand, organize,
and suggest. It does not become the product center.

### PRODUCT.2.A - Capability Output Contracts

Scope:

- [x] Standardize output contracts for `fix`, `cmd`, `explain`, `summarize`,
  and `next`.
- [x] Ensure each capability has copyable output and a predictable card shape.
- [x] Add fixtures for compiler errors, stack traces, config snippets, long
  text, and command-generation requests.
- [x] Fail gracefully on invalid model JSON, provider errors, and empty input.

Done:

- Cards never expose raw parser failures as the main answer.
- Each capability has unit tests for happy path and malformed output.
- Output shape is stable enough for keyboard actions and future automation.

### PRODUCT.2.B - Error-To-Action

Scope:

- [x] Accept compiler output, stack traces, and command failures as `fix` input.
- [x] Return likely cause, repair steps, and suggested commands.
- [x] Keep suggested commands as command cards with risk and rationale.
- [x] Avoid auto-edit and auto-run behavior.

Done:

- Rust, TypeScript, npm, and cargo error fixtures produce actionable next steps.
- Suggested commands are explicitly copy-only; risk remains display metadata.

### PRODUCT.2.C - Command Generation Safety

Scope:

- [x] `cmd` returns `command`, `risk`, `rationale`, and copy-only output.
- [x] Risk is shown clearly without creating an execution path.
- [x] Generated commands show cwd/shell/OS assumptions.
- [x] LLM output is treated as untrusted display data.

Done:

- Generated command output has no run/edit affordance.
- Copy-only flow is fast; risk remains visible and informational.

### PRODUCT.2.D - Local Context Source Display

Scope:

- [x] Show sources when a capability uses local context.
- [x] Avoid reading large private content unless explicitly selected or already
  in the active context bundle.
- [x] Keep cloud-provider memory/context injection disabled unless policy
  explicitly changes.

Done:

- Users can tell what local material informed an answer.
- Cloud and local model behavior follows the documented privacy boundary.

### PRODUCT.2.E - Next Suggestions

Scope:

- [x] Use recent workspace actions, command success, and context hash to suggest
  next steps.
- [x] Keep suggestions as replay descriptors, not automatic execution.
- [x] De-duplicate obvious noise and stale suggestions.

Done:

- Empty or low-query palette states can show useful next actions.
- Suggestions never execute without explicit user action.

## PRODUCT.3 - v0.8 Trusted Release

Goal: make release trust part of the product, not a packaging afterthought.

Scope:

- [ ] Add or validate auto-updater behavior and rollback/failure messaging.
- [ ] Complete code signing and platform notarization/signature requirements.
- [ ] Add diagnostics export for logs, config redaction, version, feature flags,
  and indexing/search state.
- [ ] Exercise config migration and rollback paths.
- [ ] Keep `docs/security.md`, README, ADRs, and release notes synchronized for
  any security, IPC, secret, or network-policy change.
- [x] Make release workflow run verify before packaging. If full verify is too
  slow, minimum gate is lint, frontend build, frontend tests, Rust tests, and
  clippy.

Done:

- A user can understand what is installed, what can connect to the network, and
  how secrets are stored.
- Release notes describe user-visible changes, not just commits.
- Security docs match actual behavior for keychain, network allowlist, CSP, IPC,
  and risky actions.

Validation commands:

```bash
npm run lint
npm run build
npm run test
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```

## PRODUCT.4 - v0.9 Daily-Use Workflow

Goal: make Keynova useful enough that the developer can use it continuously for
daily work.

Scope:

- [ ] Improve workflow memory ranking with recency, frequency, success rate,
  workspace, and context hash.
- [ ] Add workspace profiles for project-specific commands and preferences.
- [ ] Add command replay for common safe workflows with clear provenance.
- [ ] Polish cross-platform UX after the Windows-first path is stable.
- [ ] Run a 7-day dogfood pass and record friction by workflow stage.

Done:

- Recent successful workflows influence search and `next` in a visible way.
- Command replay remains inspectable and risk-gated.
- Core daily flows show zero crashes during the dogfood window.
- Cross-platform limitations are documented instead of hidden.

## PRODUCT.5 - v1.0 Public Stable

Goal: publish a stable, trustworthy, focused product.

Scope:

- [ ] Complete README, `docs/user-guide.md`, `docs/developer-guide.md`,
  `docs/security.md`, `docs/roadmap.md`, and `CHANGELOG.md`.
- [ ] Confirm the core workflow can be completed entirely from the keyboard.
- [ ] Confirm file, shell, API key, LLM output, indexing, and network trust
  boundaries are documented and enforced.
- [ ] Prepare install/release notes that explain trust, limitations, and the
  non-goal of autonomous agent behavior.
- [ ] Burn down launch-blocking bugs from v0.6-v0.9 dogfood.

Done:

- New users can install, understand the product, and complete the core workflow
  without reading source code.
- Core workflow is stable, keyboard-complete, and approval-aware.
- The public positioning matches actual product behavior.

## Parked Or Optional Features

These features are not rejected, but they should not expand until the workflow
core is stable and the freeze is lifted:

- Model Manager
- Translation
- Notes
- Automation Pipeline
- Nvim Integration
- Learning Panel
- System Monitor
- Plugin System
- Long-term autonomous agent memory

Treatment:

- Existing surfaces may remain behind feature gates.
- Do not make these the product narrative for v0.6/v0.7.
- Reassess each feature after the core workflow KPIs pass.

## KPI And Acceptance Targets

- Speed: `Ctrl+K` to input-ready should feel under 200 ms on the primary target
  machine; low-end machines must not show obvious launch jank.
- Search: top 3 results should contain the expected item in more than 80% of
  core workspace scenarios.
- Keyboard: MVP primary workflows should require zero mouse operations.
- Safety: high-risk actions should have zero known confirmation bypass paths.
- Stability: core workflows should show zero crashes over a normal dogfood week.
- Retention: the developer should use Keynova for at least 7 consecutive days
  before public-stable positioning.
- AI usefulness: track whether AI output is copied or run to determine whether a
  capability is earning its place.

## Major Risks And Mitigations

- Scope creep: separate MVP, later, and parked features; v0.6 only hardens core
  workflow.
- AI becomes the product center: keep single-step capabilities and do not revive
  autonomous agent behavior as the mainline.
- Search quality is not good enough: invest in ranking, workspace context, and
  workflow memory before adding broad surfaces.
- Install trust is weak: prioritize signing, notarization, release notes, and
  security docs before a broader release.
- Cross-platform behavior drifts: make Windows the first high-quality platform,
  then document and fix macOS/Linux gaps deliberately.
- Security docs drift from implementation: update README/security docs/ADRs for
  every security, IPC, secret, network, or high-risk action change.
