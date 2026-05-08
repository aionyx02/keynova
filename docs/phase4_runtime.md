# Phase 4 Runtime Notes

## IPC Boundary

Search now returns UI-facing items with `ActionRef` values. Backend action payloads stay in `ActionArena` and are resolved by `action.run`. Secondary actions are lazy: the UI calls `action.list_secondary` only when needed.

Debug observability records IPC payload sizes, serialization timing, command latency, search latency, action execution latency, DB request latency, idle memory, idle CPU, and executable size. Payload contents are not logged.

## Knowledge Store

SQLite access is isolated behind `KnowledgeStoreHandle`. The connection lives on a dedicated worker thread, writes use a bounded channel, read responses use oneshot replies with timeout, and high-frequency fire-and-forget logs are dropped rather than blocking UI if the queue fills.

Tables created in the first migration:

- `actions`
- `action_logs`
- `clipboard_items`
- `notes_index`
- `ai_conversations`
- `workspace_contexts`
- `search_index_metadata`
- `workflow_definitions`

SQLite is configured with WAL, `busy_timeout`, and foreign keys.

## Search

Phase 4 introduces a provider registry abstraction and extends launcher search to include App, File, Command, Note, History, and Model items. Results are merged by score and capped by the UI limit. Each query receives a generation and session-scoped action refs.

