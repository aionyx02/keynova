# Architecture

The module tree, IPC routes, event topics and config keys are **not** listed
here. `ls src-tauri/src`, a grep for `impl CommandHandler`, and the config struct
answer those correctly and permanently; a copy in Markdown answers them as of
whenever someone last edited it. What follows is only the shape you cannot
recover by reading one file at a time.

## Layers, and the one rule that holds them apart

React (WebView) → Tauri IPC → core framework → handlers → managers → storage →
platform.

The rule worth stating: **a component never touches a Tauri API directly.** All
backend calls go through the single `ipcDispatch(route, payload)` entry, which
lands on `cmd_dispatch` and is routed by `CommandRouter` to one
`CommandHandler` per namespace. Handlers hold no business logic; managers are
plain Rust with no Tauri dependency, which is why they are unit-testable.
Platform differences are isolated behind `#[cfg(target_os = ...)]` in
`platform/`, never branched inline at a call site.

## Extension without touching core

This is the point of the design: a new feature is added through one of these,
and core is not modified.

| Mechanism | How you extend it |
| --- | --- |
| `CommandRouter` | implement `CommandHandler`, register it |
| `EventBus` | `publish(AppEvent::new(topic, payload))`; frontend `listen()` |
| `ConfigManager` | add a key; the handler reads it at init |
| `SearchRegistry` | implement `SearchProvider` |
| `BuiltinCommandRegistry` | implement `BuiltinCommand` |
| `PanelRegistry.tsx` | add a `React.lazy` mapping for a `Panel(name)` result |

Since ADR-0044 a feature declares its own wiring in a `register()` entry point
and central assembly iterates a manifest, so deleting a feature is deleting its
module plus one manifest line.

## Two request paths, deliberately different

**Synchronous** — the common case. `ipcDispatch` → `cmd_dispatch` → router →
handler → manager → `Result<Value, String>`.

**Streamed** — anything slow. The manager publishes to the EventBus (tokio
broadcast, capacity 256); a relay loop re-emits to the frontend. Note the topic
rename at that boundary: dotted in Rust, dashed in Tauri.

Search uses both at once, and this split is the design, not an accident: a fast
synchronous path returns apps and non-file results on the IPC thread
immediately, while file search runs on a single background worker whose results
arrive as chunk events. That worker keeps at most one task pending and one
running; a new query cancels the previous one. See Traps in `decisions.md`.

## Actions are short-lived references, not payloads

A search result carries an `ActionRef { session_id, generation, id }` rather
than an executable payload. The frontend hands it back to `action.run` and
`ActionArena` resolves it to a `LaunchPath` / `CommandRoute` / `OpenPanel` /
`Inline` / `Noop`. A ref from a stale session or generation no longer resolves —
which is what stops a result list from outliving the state that produced it.

## Storage

Config is TOML in the roaming app dir; everything else is under the local app
data dir — `knowledge.db` (SQLite, accessed through an async actor behind
`KnowledgeStoreHandle`), `notes/`, the Tantivy index, and the startup preflight
snapshot. Linux and macOS use `~/.config/keynova/` and
`~/.local/share/keynova/`.

Schema migrations are additive only: a nullable column added by an idempotent
`ALTER TABLE` guarded by `PRAGMA table_info`, never a rewrite. Legacy rows stay
readable and the pre-migration backup is the rollback.

## Startup

Boot must not block first paint. A background preflight collects local-only
facts once per OS boot — path resolution, hardware facts for model
recommendation, local Ollama reachability — and persists a snapshot that later
launches read instead of paying the cold path. It rebuilds on schema, app
version, boot ID, source mode, or Ollama URL change. Packaged launches and
`tauri dev` share the same path; nothing may depend on installer-only hooks.
