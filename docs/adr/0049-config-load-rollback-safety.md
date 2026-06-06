---
type: adr
status: proposed
priority: p1
updated: 2026-06-06
context_policy: retrieve_only
owner: project
---

# Config Load Rollback Safety (Corrupt-Config Quarantine)

**Status:** proposed
**Date:** 2026-06-06
**Decision makers:** AI agent draft; developer acceptance requested by PRODUCT.3
**Related documents:**

- `docs/tasks/product-roadmap.md` (§PRODUCT.3)
- `docs/adr/0010-config-manager-toml.md`
- `docs/security.md`

---

## 1. Context

PRODUCT.3 (trusted release) calls for exercising config migration and rollback
paths. The one-shot legacy path migration (`app/migration.rs`) already exists, but
config **load** had a silent data-loss gap: `ConfigManager::new()` fell back to
defaults whenever `config.toml` failed to parse, with no copy of the original.

A transient corruption (partial write on crash, disk issue, a hand-edit typo)
therefore discarded every user setting, and the next `set()` → `persist()` would
overwrite the corrupt file with defaults — making the loss permanent and
unrecoverable. There was no rollback path for the user.

## 2. Decision

When the user `config.toml` exists but cannot be read/parsed, **quarantine** the
original before falling back to defaults:

- `ConfigManager::load_or_recover` copies the unparseable file to
  `config.toml.corrupt-<unix_secs>` (same app-owned config dir), logs it, then
  loads defaults. A missing file keeps the existing default fallback with no
  backup. Quarantine is best-effort: copy failure is logged, never fatal.

This preserves the user's data for manual recovery and stops the
overwrite-with-defaults data loss. No config format/schema change, no new IPC, no
network. The backup lives in the same directory and trust zone as `config.toml`
itself, so it adds no new exposure surface.

Deliberately **out of scope** (deferred, YAGNI): a forward-versioned config
migration framework (`config_version` + ordered migrations). No real config
format migration exists today; the existing plaintext→keychain secret migration
and the legacy path migration already cover current needs. A versioning seam will
get its own ADR when an actual format change requires it.

Rejected alternative: keep silently loading defaults. Simplest, but loses user
settings with no recovery path — the exact failure PRODUCT.3 wants closed.

## 3. Consequences

- A corrupt config no longer destroys user settings; the original is recoverable.
- One extra local file may appear (`config.toml.corrupt-*`) on a corruption event;
  it is not cleaned up automatically (rare, and deletion is the user's call).
- The quarantined file may contain whatever the corrupt config held (including a
  plaintext secret if corruption happened mid-migration); it stays local in the
  already-protected config dir — no new disclosure beyond on-disk `config.toml`.
- Behavior is unchanged for valid and missing configs.

## 4. Rollback

Revert `load_or_recover`/`backup_corrupt_config` so `new()` falls straight back to
defaults. No persisted data or schema migration is involved.
