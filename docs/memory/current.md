---
type: working_memory
status: active
priority: p0
updated: 2026-05-29
context_policy: always_retrievable
owner: project
---

# Current Project Memory

## Current Strategy

- Treat Keynova as a keyboard-first workflow tool, not an AI chat product.
- Make unified search/result handling the product spine.
- Move AI from product core to a stateless capability layer surfaced inline from the palette.
- Keep the docs system retrieval-first: startup state stays small, and detailed plans live in on-demand task files.

## Current Focus

- P0 is the ADR-0029 / AI capability refactor track from `docs/tasks/refactor-ai-capability.md`.
- `PREFLIGHT` (2026-05-29 P0 stability override) owns the startup preflight snapshot + once-per-boot refresh path so `/model_download` stops paying the cold path; runtime landed, validation/docs cleanup pending.
- REF.6 closed at unit level 2026-05-29 (merged into `dev` as `31786eb`). All 5 capabilities live (`explain` / `summarize` / `fix_error` / `gen_command` / `suggest_next`); palette is a prefix + NL-fallback dispatcher; src layout migrated to feature-first + `src/shared/`; ADR-0040 (slim template) proposed.
- Current branch `feature/ref-7-quantitative-gates` opens REF.7. Sub-batches: .A flag default + schema + legacy mount gate, .B size guard + bench harness, .C release notes + ADR measurement appendix, .D user-action qwen2.5:7b bench reading.
- `agent_runtime.rs < 400` REF.7 criterion is dropped — file does not exist in repo. Documented in the REF.7 plan as the same class of docx-vs-repo divergence as the `CommandPalette.tsx < 250` deviation.
- `current.md` and `active.md` are current-state indexes only. REF.6 batch detail lives in `docs/memory/sessions/2026-05-29.md`.

## Important Constraints

- AI agents can draft ADRs as `proposed`; ADR-0039 must remain `proposed` in docs even though the developer explicitly approved the bounded startup-preflight runtime implementation on 2026-05-29.
- Freeze new feature work before `REF.7` unless it is required by the refactor, fixes a P0 regression, or protects a documented safety boundary.
- LLM-driven execution must stay approval-gated for risky or system-affecting actions; risk + `ConfirmRequirement` live on `UnifiedResult.ActionChip`.
- Generic shell tool exposure stays blocked until the platform sandbox boundary is complete.
- Background Core memory targets exclude active WebView, loaded local LLM model memory, PTY terminal sessions, monitoring streams, and index rebuild tasks.
- Three intentional deviations from docx literal text remain in force: `CommandPalette.tsx < 250` is dropped, `AiPanel.tsx` physical deletion is deferred to `REF.8`, and inline AI invocation uses capability cards in the palette body rather than row chips; entry is now mixed (`explain` / `summarize` / `fix` stay explicit prefixes, while `next` empty-state mount and `cmd` NL-query detection are enabled).

## Next Step

- Land REF.7.A (`ai.legacy_agent` schema + flag-guarded legacy mount) on this branch.
- Land REF.7.B (`scripts/code-guard-size.mjs` + `scripts/bench-ai-capability.mjs`) as a separate commit.
- REF.7.C waits on REF.7.D (formal qwen2.5:7b reading) + user-side Bug A/B smoke.
- REF.7.D is user-action: `ollama pull qwen2.5:7b && npm run bench:ai -- --runs 10 --model qwen2.5:7b`.
- `PREFLIGHT` follow-through still needs validation/docs cleanup.
- Run `npm run docs:refresh` before commit or handoff.
