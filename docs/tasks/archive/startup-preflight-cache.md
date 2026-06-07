---
type: task_plan
status: active
priority: p0
updated: 2026-05-29
context_policy: on_demand
owner: project
tags: [startup, preflight, cache, p0, performance, model-download]
---

# P0 Stability: Startup Preflight Snapshot And Once-Per-Boot Refresh

Source: user-mandated P0 insertion on 2026-05-29 to stop `/model_download`
cold-path crashes / flash exits and to move expensive one-time detection off the
interactive path.

Goal: on first app launch after install or source-run startup, collect the
small set of local bootstrap metadata Keynova needs (paths, app-owned icons,
hardware, model bootstrap facts), persist it under the local `Keynova/`
directory, and refresh it once per OS boot instead of recomputing the same
expensive probes on every panel open.

Implementation status (2026-05-29):

- `PREFLIGHT.1` to `PREFLIGHT.4` are now implemented in runtime code under the
  explicit developer approval given in the 2026-05-29 conversation.
- ADR-0039 remains `proposed`; the implementation must not flip ADR status
  automatically.
- `PREFLIGHT.5` is partially landed: observability, manual refresh IPC, and
  unit coverage exist; broader smoke coverage remains.

## Problem Statement

Current cold-path issues:

- `ModelDownloadPanel.tsx` kicks off `model.detect_hardware` and
  `model.recommend` on mount; the latter also triggers a background catalog
  refresh path.
- Hardware detection is still done synchronously through platform probes, so
  the first `/model_download` open pays the full detection cost on the
  interactive path.
- Keynova already has scattered startup warmups (`prescan_apps`, file index,
  terminal prewarm, per-icon disk cache), but no single persisted "bootstrap
  snapshot" for required local metadata.
- Missing or slow local prerequisites currently degrade ad hoc per feature,
  which increases startup jitter and makes `/model_download` especially fragile.

## Scope And Intent

This track is intentionally bootstrap-only. It is not a generic indexing
project and not a new agent/local-context expansion.

In scope:

- App-owned icon asset verification
- Keynova directory/path resolution and creation
- Hardware snapshot needed by model recommendation
- Local model bootstrap facts (`ollama_url`, reachable/not reachable, local tag
  list when cheap enough, recommended models derived from cached hardware)
- Persisted once-per-boot snapshot plus invalidation rules
- `/model_download` consuming snapshot-first state instead of doing full cold
  detection on mount

Out of scope:

- Recursive scanning of arbitrary user content
- Learning-material / local-context file ingestion
- Remote model catalog fetch on startup
- Pre-extracting every filesystem shell icon at boot
- Installer custom actions; packaged build and `tauri dev` must use the same
  first-launch bootstrap path

## Proposed Design

### 1. Trigger Model

- Run one shared startup preflight runner from the Tauri setup path used by
  both packaged app launches and source/dev launches.
- Do not put this logic into NSIS/MSI/post-install scripts. The installed app
  and `npm run tauri dev` should exercise the exact same bootstrap flow.
- The runner must be backgrounded and non-blocking for first paint. Features
  consume the last completed snapshot and subscribe to updates instead of
  waiting on boot probes inline.

### 2. Storage Model

Persist a small JSON snapshot under a Keynova-owned local directory resolved by
`platform_dirs` (exact data-vs-cache placement to be locked by ADR):

- Proposed path family: `Keynova/bootstrap/preflight-v1.json`
- Optional lock/status sidecar: `Keynova/bootstrap/preflight-v1.state.json`

Suggested snapshot shape:

```json
{
  "schema_version": 1,
  "app_version": "0.2.0",
  "boot_id": "platform-boot-marker",
  "source_mode": "packaged|dev",
  "status": "ready|partial|failed|running",
  "generated_at": "2026-05-29T12:34:56Z",
  "paths": {
    "config_dir": "...",
    "data_dir": "...",
    "notes_dir": "...",
    "search_index_dir": "...",
    "nvim_dir": "...",
    "icon_cache_dir": "..."
  },
  "hardware": {
    "ram_mb": 16384,
    "vram_mb": 8192,
    "cpu_cores": 16
  },
  "icons": {
    "bundled_assets_ok": true,
    "missing_assets": []
  },
  "model": {
    "ollama_url": "http://localhost:11434",
    "ollama_reachable": false,
    "local_models": [],
    "recommended_models": ["qwen2.5:3b", "phi4-mini:3.8b"]
  },
  "warnings": [],
  "errors": []
}
```

### 3. Once-Per-Boot Invalidation

- Re-scan when boot ID changes, snapshot is missing, schema version changes, or
  the app version crosses an explicit invalidation boundary.
- Proposed boot marker: `sysinfo::System::boot_time()` plus platform-specific
  normalization if needed.
- Same boot + same schema should reuse the persisted snapshot and skip the
  expensive probes.

### 4. Two-Tier Boot Work

To avoid moving lag from `/model_download` into app startup, split the work:

Tier A — must stay cheap and always run:

- Resolve/create Keynova directories
- Validate app-owned icon files / icon cache directory presence
- Load previous snapshot and decide whether a rescan is required

Tier B — once-per-boot background refresh:

- Detect RAM / VRAM / CPU cores
- Read `ai.ollama_url`
- Probe local Ollama reachability with a short timeout
- Optionally fetch local Ollama tags when reachable
- Compute recommended model list from cached hardware

Tier C — remain lazy/on-demand:

- Remote `ollama.com/library` catalog refresh
- Full filesystem shell icon extraction
- Recursive file index rebuild beyond the existing startup index rules

### 5. Consumer Contract

`/model_download` should switch to snapshot-first consumption:

- `model.detect_hardware` becomes snapshot-backed with a live fallback only when
  the snapshot is missing or explicitly stale.
- `model.recommend` should use preflight hardware + cached local facts and keep
  remote catalog refresh off the mount-critical path.
- The panel must tolerate `status = partial|failed` and render diagnostics
  instead of crashing or flashing closed.

Potential contract additions:

- `model.preflight_status`
- `model.bootstrap_snapshot`
- EventBus topic such as `startup.preflight.updated`

## Non-Goals

- No new generic shell probing or privileged installer actions
- No startup network call to third-party hosts outside the already approved
  local Ollama reachability check
- No background ingestion of user private files
- No promise that every app/file search icon is precomputed before first search

## Task Breakdown

### PREFLIGHT.0 — ADR + boundary lock

Scope:

- Draft a new ADR for startup preflight snapshot storage, boot invalidation,
  and allowed probe surface.
- Lock whether the snapshot lives under `platform_dirs::keynova_data_dir()` or
  a cache sibling.
- Reconcile architecture/security docs with the final allowed local path list.

Done:

- Proposed ADR exists.
- `docs/security.md` / `docs/architecture.md` update plan is explicit.
- ADR-0039 remains `proposed`; implementation is proceeding only because the
  developer explicitly approved runtime work in the 2026-05-29 thread.

### PREFLIGHT.1 — Snapshot schema + storage helpers

Scope:

- Add a dedicated Rust module for snapshot structs, load/save helpers, and boot
  ID comparison.
- Normalize Keynova-owned directory creation in one place.
- Introduce schema versioning and partial-failure recording.

Done:

- Snapshot file can be read/written safely.
- Missing dirs are created before feature code needs them.
- Corrupt snapshot falls back to rebuild instead of panic.

### PREFLIGHT.2 — Startup runner + once-per-boot refresh

Scope:

- Hook the runner into the shared app startup path.
- Add singleflight behavior so only one refresh happens per process.
- Publish update/failure events for UI consumers.

Done:

- Packaged app and `tauri dev` both execute the same startup preflight flow.
- Same-boot relaunch reuses the persisted snapshot.
- New boot forces one background refresh.

### PREFLIGHT.3 — `/model_download` hardening

Scope:

- Change the panel and model IPC to consume snapshot-backed bootstrap data.
- Remove mount-critical synchronous dependency on full live hardware detection.
- Separate remote catalog refresh from the panel's first render path.

Done:

- `/model_download` renders on cold boot without flash-crashing.
- Hardware chips and initial recommendation list can come from snapshot data.
- Offline / unreachable Ollama shows a clear warning state instead of failing
  the whole panel.

### PREFLIGHT.4 — Icon/path bootstrap integration

Scope:

- Verify app-owned icons required by launcher/Tauri surfaces.
- Ensure icon cache dir and other Keynova dirs exist before hot-path features
  touch them.
- Record missing asset/path warnings into the snapshot.

Done:

- Missing bootstrap assets degrade to warnings, not panics.
- Keynova-owned directories exist before dependent features run.

### PREFLIGHT.5 — Observability, rebuild, and tests

Scope:

- Add observability for preflight duration, snapshot reuse, and failure reason.
- Add a manual rebuild route or dev utility for forcing snapshot refresh.
- Add unit/integration/manual validation coverage.

Done:

- There is a way to force-refresh the snapshot without rebooting.
- Tests cover boot-ID invalidation, corrupt snapshot recovery, and
  `/model_download` snapshot-first load.

## File Map

Likely touch points:

- `src-tauri/src/app/bootstrap.rs`
- `src-tauri/src/app/watchers.rs`
- `src-tauri/src/app/state.rs`
- `src-tauri/src/core/observability.rs`
- `src-tauri/src/core/` new preflight module
- `src-tauri/src/platform_dirs.rs`
- `src-tauri/src/handlers/model.rs`
- `src-tauri/src/managers/model_manager.rs`
- `src/components/ModelDownloadPanel.tsx`
- `src-tauri/default_config.toml` and setting schema if a preflight flag lands

## Validation Matrix

Functional:

- First launch after install or `tauri dev` creates a snapshot under the local
  Keynova directory.
- Relaunch within the same OS boot reuses the snapshot.
- After OS reboot, the next launch performs one refresh.
- `/model_download` remains open and usable even when Ollama is offline.

Performance:

- Startup preflight does not block launcher first paint.
- `/model_download` no longer performs the heaviest cold probes on the first
  mount.

Safety:

- No recursive user-content scans are added to startup.
- No new remote third-party fetch is added to startup.
- Snapshot writes stay within Keynova-owned local directories.
