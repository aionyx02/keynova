---
type: task_plan
status: active
priority: p1
updated: 2026-06-05
context_policy: on_demand
owner: project
tags: [ux, i18n, a11y, frontend]
---

# UX.AUDIT — UI/UX consistency pass

Developer-directed (2026-06-05). Full code-level frontend walkthrough; findings
and completed work are detailed in `docs/memory/sessions/2026-06-05.md`. Branch:
`feature/personal-memory`.

## Done

- `UX.AUDIT.1` drop stale "AI Chat" naming → inline-AI wording (`cba9fd4`).
- `UX.AUDIT.2` first screen-reader live regions, `aria-live`/`role=status|alert`
  on palette search state + SettingRow/ModelPanel/NoteEditor status (`66ecab9`).
- `UX.AUDIT.3` full i18n conversion — **infra + all batches done**:
  - Infra: `src/i18n/format.ts` `fmt(template, vars)` `{token}` interpolation.
  - Batch 1 ModelPanel (`model.*`), Batch 2 SettingPanel/SettingRow (`settings.*`),
    Batch 3 five AI capability cards (`capability.*`), Batch 4 palette secondary
    actions + suggestion footers (`palette.*` + `buildSecondaryActions(result,
    labels)`), Batch 5 SearchResultsList (`search.*` kinds/fallbacks/footer).
  - Final sweep: Cheatsheet/Onboarding overlays, Terminal/Learning/SystemMonitor/
    Nvim/Preview/ErrorBoundary/Rank/FloatingWindow panels, plus command-palette,
    calculator/history/system/translation stragglers. JSX visible-text scan now
    only reports key glyphs, brands/acronyms, route/type declarations, or
    non-localized path constants.
- `UX.AUDIT.4` UTF-8 BOM cleanup — stripped the 9-file BOM list and rechecked
  that no listed file still starts with BOM.

## Open

### UX.AUDIT.5 — garbled-text (`嚙`) root cause — BLOCKED on repro

`SearchResultsList.hasEncodingError('嚙')` masks mojibake titles as "Unavailable
text". All live search-result read paths verified encoding-correct (Everything
wide `…W` APIs + `\u{FFFD}` skip; `.lnk` `file_stem().to_str()` + FFFD guard; fs
walk). No current code path produces `嚙`, so the guard is likely vestigial or
masking stale store data. **Needs a concrete garbled result (raw filename/path)
from the user to locate the source.** Fallback option if unrepro'd: show the raw
path instead of "Path unavailable" so the row stays actionable.

## Non-goals

- No new ADR (pure i18n reuse + additive locale keys).
- Don't touch `current.md`/`active.md` beyond the one-line index entry.
