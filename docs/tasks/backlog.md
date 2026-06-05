---
type: task_index
status: backlog
priority: p1
updated: 2026-06-05
context_policy: on_demand
owner: project
tags: [roadmap, docs-governance, planning]
---

# Backlog Tasks

Backlog is on-demand planning context. During the P0 refactor track, it is not a runnable queue.

## P0

- (empty; current P0 work lives in `docs/tasks/active.md` and `docs/tasks/refactor-ai-capability.md`)

## P1

- [ ] Revalidate `PRODUCT.ROADMAP` after the P0 freeze lifts. Detailed plan:
  `docs/tasks/product-roadmap.md`.
- [x] `PRODUCT.0` lock product positioning and non-goals: Keynova is a
  keyboard-first local workflow entry for technical workers, not a general
  launcher or AI chat app. Landed in README (定位與非目標 + 核心 vs 選用) and
  `docs/project.md`; docs-only, no runtime change.
- [~] `PRODUCT.1` v0.6 stable workflow core: workspace-aware search,
  transparent ranking, project command discovery, terminal workflow,
  file actions, developer utilities, and keyboard/performance gates.
  **Unfrozen 2026-06-05** and promoted to `docs/tasks/active.md` (conditional
  partial unfreeze; search-core is orthogonal to the REF.7 AI gates).
- [ ] `PRODUCT.2` v0.7 useful inline AI capabilities: `fix`, `cmd`, `explain`,
  `summarize`, and `next` with stable output contracts, source display, and
  risk-gated command handling.
- [ ] `PRODUCT.3` v0.8 trusted release: updater/signing/notarization,
  diagnostics export, config migration checks, security docs sync, and
  verify-before-package release gates.
- [ ] `PRODUCT.4` v0.9 daily-use workflow: workflow-memory ranking, workspace
  profiles, command replay, cross-platform UX fixes, and 7-day dogfood.
- [ ] `PRODUCT.5` v1.0 public stable: user/developer docs, roadmap/changelog,
  install trust, keyboard-complete core workflows, and launch-blocker burn-down.
- [ ] Reassess docs guard threshold fit after several normal refactor commits.
- [ ] Decide whether `completed.md` regeneration should stay manual or become part of `docs:refresh`.
- [ ] Review `docs/tasks/bug-followup.md`: keep reusable structural bug patterns, migrate dated narratives to sessions, and fold active regression checks into `REF.2`/`REF.7`.

## P2

- [ ] Revalidate `LAUNCH.2.C` per-workspace quick actions after `REF.7`.
- [ ] Revalidate `ONBOARD.1.D/E` re-engage prompt and hotkey guidance after `REF.7`.
- [ ] Revalidate `NOTE.1` daily note driver after `REF.7`.
- [ ] Revalidate `UTIL.1.B-online` online currency rates after `REF.7` and ADR-038 status review.

## Frozen

- Conditional partial unfreeze (2026-06-05): `PRODUCT.1` search-core is unfrozen
  and active. `PRODUCT.2` (AI capabilities) and any AI-capability/agent-touching
  feature stay frozen until `REF.7.D` lands + the observation window closes.
- `AGENT.3` and `AI.1` are superseded by the ADR-0029 / AI capability refactor track and should not be implemented as separate tracks.
- `AGENT.4`, `CLIP.1`, `SNIP.1`, `WIN.1`, `UTIL.3`, `DEV.1`, and `SYNC.1` remain parked until the refactor observation period is complete and their ADR gates are rechecked.
- Product-plan optional surfaces stay parked until core workflow KPIs pass:
  Model Manager, Translation, Notes, Automation Pipeline, Nvim Integration,
  Learning Panel, System Monitor, Plugin System, and long-term autonomous agent
  memory.
- Do not move detailed execution history back into `current.md`, `active.md`, `completed.md`, or this backlog.
- When a task group is completed, record detail in the relevant session log and let the completed index stay compact.
