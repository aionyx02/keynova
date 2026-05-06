# Event Naming and Compatibility Adapter

Phase 4 moves Keynova events toward canonical dot-separated topics. Existing React listeners still use historical Tauri event names with hyphens, so the backend emits both names during the migration window.

## Rules

- Canonical topics use dot-separated names, for example `domain.entity.action`.
- Legacy aliases replace dots with hyphens, for example `domain-entity-action`.
- New backend EventBus publishers should use canonical topics.
- Existing frontend listeners may keep legacy aliases until each component is migrated.
- Event payloads must remain JSON-serializable and should not include secrets.

## Backend Adapter

The EventBus bridge emits the canonical topic first. If the legacy alias differs, it emits the alias too. This lets new code subscribe to canonical topics while preserving old listeners.

## Current Event Map

| Canonical topic | Legacy Tauri event | Notes |
| --- | --- | --- |
| `terminal.output` | `terminal-output` | Existing terminal stream event. Future target: `terminal.session.output`. |
| `system.ping.completed` | `system-ping-completed` | Diagnostic ping event. |
| `config.reloaded` | `config-reloaded` | Config reload success. Future target is already close enough. |
| `config.reload_failed` | `config-reload_failed` | Existing underscore retained until publisher rename. Future target: `config.reload.failed`. |
| `history.updated` | `history-updated` | Clipboard/history update event. |
| `translation.result` | `translation-result` | Translation response event. |
| `ai.response` | `ai-response` | AI response event. |
| `model.pull.progress` | `model-pull-progress` | Ollama pull progress event. |
| `model.pull.done` | `model-pull-done` | Ollama pull completion event. |
| `model.pull.error` | `model-pull-error` | Ollama pull failure event. |
| `model.catalog.updated` | `model-catalog-updated` | Model catalog refresh event. |

## Direct Window Events Still To Migrate

These are emitted directly from window/shortcut code instead of through EventBus. Keep them stable until `lib.rs` is split into `app/*` and a shared emitter helper exists.

| Current Tauri event | Future canonical topic |
| --- | --- |
| `window-focused` | `window.focus.changed` |
| `mouse-control-toggled` | `mouse.control.toggled` |
| `workspace-switched` | `workspace.context.switched` |
