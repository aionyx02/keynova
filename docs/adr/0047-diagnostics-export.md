---
type: adr
status: proposed
priority: p1
updated: 2026-06-06
context_policy: retrieve_only
owner: project
---

# Diagnostics Export Bundle Contract

**Status:** proposed
**Date:** 2026-06-06
**Decision makers:** AI agent draft; developer acceptance requested by PRODUCT.3
**Related documents:**

- `docs/tasks/product-roadmap.md` (§PRODUCT.3)
- `docs/security.md`
- `docs/adr/0010-config-manager-toml.md`
- `docs/adr/0039-startup-preflight-snapshot.md`

---

## 1. Context

PRODUCT.3 (trusted release) calls for a diagnostics export so a user — or the
developer during dogfood — can describe *what is installed and configured* when
filing a bug, without hand-collecting version, flags, paths, and config. Keynova
keeps no persisted log files (observability is stderr-only in debug builds), so
there is nothing to "export" in the log-file sense; the useful artifact is a
redacted runtime/config summary.

The risk is leakage: config contains secret references and the data directory
paths embed the OS username. Emitting raw config or absolute paths into a string
the user will paste into a public tracker is a security regression. The export
is a new user-facing output/data contract, so governance (§7) requires an ADR.

## 2. Decision

Add a core `diagnostics` assembler and a `/diag` builtin command that returns an
**`Inline`** bundle rendered as deterministic plain text. The bundle contains:

- app facts: version (`CARGO_PKG_VERSION`), `std::env::consts::{OS,ARCH}`;
- feature flags: the `features.*` and `ai.legacy_agent` rows;
- redacted config: every key from `ConfigManager::list_all_redacted()`;
- local data: presence + on-disk size (metadata only, **never contents**) of
  `config.toml`, `knowledge.db`, `notes/`, the Tantivy index, and the preflight
  snapshot;
- startup preflight summary: status, source mode, ollama reachability, local
  model count, generated-at (read-only via `read_snapshot_from_disk`).

Redaction is contractual and enforced in the pure assembler:

- only `list_all_redacted()` rows are consumed (never `get()` / `list_all()`);
  sensitive rows are re-masked to `********` as defense-in-depth.
- absolute paths have the user home prefix collapsed to `~` / `%USERPROFILE%`.

The export is **copy-only**: it reuses the existing inline-result Copy + scroll
region. There is no network call, no auto-upload, no file write, and no run/edit
affordance. `/diag` is a core trust command and is **not** feature-gated.

Extension rule (minimal-scope, additive): new fields may be added to the report
struct; any new source must pass through the same redaction guarantees and add no
file-content or network behavior. A save-to-file affordance is out of scope and
would need its own ADR (new write-IPC + path boundary).

Rejected alternative: dump raw config + absolute paths. Simpler but leaks secret
references and the username, defeating the trust goal.

## 3. Consequences

- Users get a one-command, paste-safe snapshot for bug reports.
- Secrets and usernames never reach the bundle; tests assert no leak.
- The assembler is pure and unit-tested; path I/O (existence/size) stays in the
  handler and is metadata-only.
- The bundle grows with config, but stays bounded by the existing setting schema.
- No persisted data, schema, or IPC-route change; `cmd.run` is reused.

## 4. Rollback

Remove the `/diag` registration + handler branch and the `core::diagnostics`
module; revert the `read_snapshot_from_disk` accessor. No persisted data or
migration is involved, so rollback is a pure code removal.
