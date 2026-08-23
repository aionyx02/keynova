# Security

What is refused and why. Which function enforces what is in the code
(`network_policy.rs`, `secret_store.rs`, `dispatch.rs`, `diagnostics.rs`); the
current advisory posture is in CI. Neither is repeated here.

Priority order, when two of these conflict:

```
security > data correctness > rollbackability > testability > performance > speed
```

## Trust boundary

Untrusted: everything the user types, every path the user supplies, **every LLM
output**, and every network response. Trusted: Rust-internal calls, config read
through `ConfigManager`, rows read from the knowledge store.

The boundary itself is the IPC surface — payload validation in `cmd_dispatch`,
path canonicalization plus workspace-root checks, and secret redaction before
anything reaches a prompt or a preview. LLM output crossing back is still
untrusted: it is never executed, only displayed or copied.

## Standing refusals

**No generic shell execution.** Not gated behind approval, not "just for typed
commands the model produces". The product sandbox boundary is incomplete, and an
approval dialog does not make arbitrary execution safe. Deterministic typed
tools with bounded output are the supported answer. Generated commands are
copy-only.

**No recursive scanning of private files.** `FEAT.11` local-context scanning
stays off until the ADR-0028 boundary is actually met. Denylist and redaction
logic may be written and unit-tested in isolation.

**Secrets never live in config or logs.** They go to the OS keychain. Nothing
secret reaches a log line, a `/diag` bundle, a crash log, or a prompt.

**`/diag` and crash logs carry metadata only.** Presence and size, never
contents; home paths collapsed, messages capped. Redaction is contractual — a
new field must prove it is redacted, not be assumed safe.

**Production CSP carries no dev origins.** Dev origins exist only in `devCsp`.

## Changes that require an ADR first

Do not implement these and document them afterwards:

- new user-controllable read, write, delete, move, or recursive scan
- widening an existing file-access scope
- changing an allowlist, denylist, sandbox, or capability definition
- a new outbound network destination
- anything needing OS permissions (Accessibility API, global hooks)
- changing the ADR-0030 risk-tag contract or who owns confirmation
- any feature that handles the user's private files

## Agent configuration is a permission surface

Agent-executable configuration is **not tracked by default**. `.gitignore`
covers `.claude/`, `.mcp.json`, `.cursor/`, `.cursorrules`, `.amazonq/`,
`.windsurf*`, `.roo/`, `.aider*`, `.github/copilot-instructions.md` and
`AGENTS.md`. The reviewed exception is `CLAUDE.md`. Adding a new agent tool
means adding its entry too — **a missing ignore rule is a default-allow**.

Two hard rules:

- A PR touching `CLAUDE.md` or any agent configuration is reviewed as a
  **permission change**. The "documentation changes merge quickly" convention
  does not apply to it.
- An AI agent may not widen its own permissions: it does not edit `.claude/`,
  `capabilities/default.json`, or this section.

Before taking over an outside branch or PR:

```bash
git ls-files | grep -Ei '(^|/)(\.mcp\.json|\.claude/|\.cursor/|\.amazonq/|\.windsurf|\.aider|AGENTS\.md|CLAUDE\.md)'
```

## Minimum bar before merging

CI green, no new CodeQL alert, and anything on the ADR list above carries its
ADR. Known accepted gaps are listed in `state.md` — they are decisions, not
things to quietly fix in an unrelated PR.
