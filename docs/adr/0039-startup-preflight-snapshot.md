---
type: adr
status: proposed
priority: p0
updated: 2026-05-29
context_policy: retrieve_only
owner: project
---

# Startup Preflight Snapshot

**Status:** proposed  
**Date:** 2026-05-29  
**Decision makers:** AI agent draft; developer acceptance required  
**Related documents:**
- `docs/tasks/startup-preflight-cache.md`
- `docs/architecture.md`
- `docs/security.md`

---

## 1. Context

`/model_download` currently performs hardware detection and recommendation work
on the interactive path. Keynova already has several separate warmup/cache
mechanisms, but no single persisted bootstrap snapshot for required local
metadata such as paths, app-owned icons, and model bootstrap facts.

The user requested a new P0 track that:

- runs on packaged launches and source/dev launches,
- scans the required local bootstrap inputs once,
- stores the result under the local `Keynova/` directory, and
- refreshes it once per OS boot to avoid repeated cold-path stalls.

This changes startup/background behavior, introduces a new persisted local
snapshot, and touches path/security boundaries, so it needs ADR review before
runtime implementation.

## 2. Decision

If accepted, Keynova will add a startup preflight snapshot system that:

- runs from the shared Tauri startup path used by packaged and dev builds,
- persists a schema-versioned snapshot of bootstrap metadata under a
  Keynova-owned local directory,
- invalidates the snapshot once per OS boot (plus schema/app-version rules),
- keeps startup non-blocking by splitting cheap path/icon checks from heavier
  hardware/model probes, and
- makes `/model_download` consume snapshot-first state instead of re-running the
  full cold probes on mount.

Planned scope is local-only bootstrap metadata. Remote model catalog refresh,
recursive private-file scans, and mass shell-icon precomputation remain outside
the boot path.

## 3. Consequences

Positive:

- Moves repeated cold probes off the `/model_download` interactive path.
- Gives startup-dependent features one shared snapshot contract instead of ad
  hoc warmups.
- Makes once-per-boot refresh explicit and observable.

Negative / tradeoffs:

- Adds a new persisted local file/schema that must be versioned and recoverable.
- Introduces a startup worker whose scope must be tightly bounded to avoid
  becoming a hidden indexer.
- Requires careful path/security documentation updates before implementation.

## 4. Rollback

- Remove the startup preflight module and snapshot file.
- Restore model bootstrap behavior to live detection on demand.
- Keep existing per-feature caches (such as search icon disk cache) independent
  so rollback can be limited to the shared snapshot layer.
