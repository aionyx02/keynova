# ADR Template Slim — 4-Section Default

**狀態：** 提議
**日期：** 2026-05-29
**決策者：** 開發者 / AI agent
**相關文件：**
- docs/adr/0000-template.md
- docs/decisions.md
- docs/CLAUDE.md

---

## 1. Context

The current ADR template (`docs/adr/0000-template.md`) has 9 sections: Context,
Constraints, Alternatives Considered, Decision, Consequences, Implementation
Plan, Rollback Plan, Validation Plan, and Open Questions. Every section is a
top-level heading with sub-bullets, and the template encourages enumerated
alternatives with full performance-analysis blocks per option.

In practice, this load was paid every time we landed a refactor batch
(`REF.0`–`REF.5` so far):

- **Alternatives** is usually written *after* the decision and reads like a
  retrospective justification rather than a real branch point. The same
  information lives more naturally in the PR description, where it belongs to
  the reviewer's mental model rather than a stable architecture document.
- **Implementation Plan** drifts the moment work starts. The task file
  (`docs/tasks/<feature>.md`) holds the live plan; the ADR copy goes stale
  within a day.
- **Validation Plan** has the same drift problem. The actual validation matrix
  lives in `docs/tasks/<feature>.md` or in a test scaffold; the ADR copy is
  documentation of intent only.
- **Open Questions** ends up empty or rotates into the task file as TODO items.

The cost is real: ADR-0029 (AI Capability Layer) took multiple revisions to
stabilize because every section invited churn. ADR-0030 (Backend Risk Tag
Contract) shipped at 7.9 KB partly because the template asks for
section-by-section completeness even when the decision is small.

REF.6 has been ramping the AI agent's authoring load: ADRs now get drafted by
both humans and the agent, and the 9-section default keeps adding sections
that immediately rot. Documentation rot has explicit guard wiring
(`docs/narrative-check`, `docs/guard-size`); the ADR template is one of the
remaining places where rot is structurally encouraged.

## 2. Decision

Adopt a 4-section default template for new ADRs:

1. **Context** — what is the problem and the constraints that bound the
   decision. Merges the current "Constraints" content into the same prose.
2. **Decision** — the choice itself, including the one alternative that was
   rejected with a one-line reason. (No full alternative-analysis matrix.)
3. **Consequences** — positive and negative downstream effects, including
   migration impact and risks accepted. Replaces both "Consequences" and the
   forward-looking parts of "Implementation Plan".
4. **Rollback** — how to back out, including any flag, schema, or data
   considerations. Kept because rollback is the section most often referenced
   *after* acceptance (see `safety > correctness > rollbackability` ordering
   in `docs/CLAUDE.md` §1).

What moves out of the ADR template:

- **Alternatives Considered** → PR description guidance. The PR template can
  carry a "What other options did you consider?" prompt; reviewers see it once,
  in context, and it does not pollute the long-term ADR record.
- **Implementation Plan** → `docs/tasks/<feature>.md` (the existing task plan).
  This is where live execution state belongs anyway.
- **Validation Plan** → same task file, alongside the implementation steps.
  Validation lives next to what is being validated, not next to the
  architectural decision.
- **Open Questions** → `docs/tasks/<feature>.md` as TODO bullets, or
  `docs/tasks/blocked.md` for items waiting on developer answers.

Existing ADRs are **not** retroactively rewritten. The slim template is the
default for new ADRs only. Older ADRs stay in their 9-section form because
rewriting them adds churn without adding clarity (per the same minimal-scope
preference that motivates this ADR).

Section ordering follows the canonical Context → Decision → Consequences →
Rollback flow used by the original Michael Nygard ADR template, so future
readers without Keynova-specific context can still parse the file.

## 3. Consequences

### Positive

- New ADRs ship faster. Estimate: roughly half the bytes per ADR, mostly from
  collapsing Alternatives + Implementation + Validation + Open Questions into
  the right adjacent documents.
- The agent can draft new ADRs as `提議` without producing speculative
  alternative-analysis prose that the developer has to either accept or rewrite.
- Stale-section rot stops accumulating: Implementation/Validation churn now
  lives in the task file, which is *expected* to change and has explicit guard
  rules for narrative accumulation.
- Reviewers and future maintainers can scan an ADR in one screen instead of
  scrolling through nine.

### Negative / Technical Debt

- Mixed format across the repo: ADRs 0001–0030/0038/0039 keep the 9-section
  shape; ADRs from 0040 onward use the 4-section shape. A reader skimming the
  ADR directory will see two styles. Mitigated by `docs/decisions.md` which
  already serves as the canonical index — the index is the entry point, not
  the raw directory listing.
- PR description guidance for "Alternatives" is not enforced by tooling. If a
  reviewer wants alternatives documented and the author skipped them, the
  feedback loop happens at PR review time, not at ADR authoring time.
- "Constraints" being merged into Context means new authors must remember to
  state platform / memory / latency bounds inline. Mitigated by template
  comment hints.

### For Users

- No direct effect. ADRs are a developer-facing artifact.

### For Developers

- One fewer ritual per architecture-touching change.
- The PR template becomes a more important artifact than before; it is now
  the home for alternatives discussion.

### For Tests

- `docs/guard-size` already gates `current.md` / `active.md` size; no new ADR
  size gate is introduced by this ADR. ADRs can still grow when the decision
  is complex.
- `docs/guard-schema` will need a follow-up to accept the slim front-matter
  shape if/when ADR front-matter ever becomes mandatory. For now, ADR files
  carry no front-matter (consistent with current ADRs).

### For Security

- No change. Decisions that affect security boundaries still get full
  consequence analysis in section 3; the slim template does not drop that
  surface.

## 4. Rollback

Rollback is cheap because nothing executable depends on this ADR:

- **No code changes.** The slim template only affects new authoring; no
  runtime behavior depends on it.
- **No data migration.** Markdown files are independent of each other.
- **Revert path:** mark this ADR `廢棄` in `docs/decisions.md` and instruct
  future ADRs to revert to `0000-template.md`. ADRs authored under the slim
  template stay in their slim form (do not retroactively expand) — they
  remain valid 4-section ADRs even after the default reverts.
- **Validation that rollback worked:** the next ADR drafted after the revert
  uses the 9-section format and passes `docs:guard`.

This ADR is itself authored in the slim 4-section format as a dogfood
demonstration; if the slim template proves insufficient for any practical ADR
size in the next observation cycle, this ADR becomes the first candidate to
revert.