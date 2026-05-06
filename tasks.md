# Keynova Tasks

> Compact task board. Keep this file short: completed work is summarized, remaining work is grouped into executable batches.
> Update `memory.md` and `decisions.md` when a batch changes architecture or runtime behavior.

## Current Baseline

- [x] Phase 0-3.8 foundation: Tauri/React/Rust scaffold, CommandRouter/EventBus, launcher UI, app/file search, terminal panel, settings, hotkeys, mouse/system controls, AI/translation/workspace/note/calculator/history/model features.
- [x] BUG-1 to BUG-15 user-facing fixes, including first-open lazy panel sizing/scrollbar refresh.
- [x] Phase 2 search backend debug/config/rebuild/test path, with Tantivy still pending as the offline file provider.
- [x] Phase 4 foundation: ActionRef/ActionArena, UiSearchItem, action.run, secondary actions, schema-driven settings, Workspace Context v2 base, Knowledge Store DB worker, Agent mode skeleton, Automation/Plugin security docs.
- [x] Knowledge Store batch writes, bounded queue/drop policy, shutdown flush, DB observability.
- [x] Plugin manifest loader and deny-by-default permission parser.
- [x] Agent read-only tool runtime: `agent.tool`, `keynova.search`, SearXNG-backed `web.search`, visibility filtering, grounding sources, search cancel/generation checks.

Last full verification baseline: `npm run build`, `npm run lint`, `cargo test`, `cargo clippy -- -D warnings`, `git diff --check`.

## Batch 1 - Automation And App Structure

- [x] Phase 4.6 Action chain executor.
- [x] Phase 4.0 Split `src-tauri/src/lib.rs` into `src-tauri/src/app/*` modules (`state.rs`, `bootstrap.rs`, `shortcuts.rs`, `tray.rs`, `window.rs`, `watchers.rs`, `control_server.rs`).

## Batch 2 - Search Runtime

- [ ] Phase 4.1 Search chunked/progressive streaming so first visible batch returns under 50ms.
- [ ] Phase 4.1 First batch cap/paging: return first 10-20 items, then page/lazy fetch up to 50+.
- [ ] Phase 4.1 Icons, preview, and metadata lazy load.
- [ ] Phase 4.4 Progressive results over Tauri channel chunks.
- [ ] Phase 4.4 Plugin/provider timeout boundary.
- [ ] Phase 2/4.4 Tantivy offline file provider.

## Batch 3 - Workspace And Knowledge

- [ ] Phase 4.3 Bind terminal sessions to current workspace.
- [ ] Phase 4.3 Bind notes to current workspace.
- [ ] Phase 4.3 Bind AI conversations to current workspace.
- [ ] Phase 4.3 Workspace-weighted clipboard/history ranking.
- [ ] Phase 4.5 Recent/frequent search ranking memory cache.
- [ ] Phase 4.5 Migration and backup path for Knowledge Store.

## Batch 4 - Agent Runtime

- [ ] Phase 4.5A Medium-risk approved actions: open panel, create note draft, update setting draft, run safe built-in command.
- [ ] Phase 4.5A High-risk action scaffolding with explicit approval: terminal command, file write, system control, model download/delete.
- [ ] Phase 4.5A Agent memory layers: session, workspace, long-term with opt-in.
- [ ] Phase 4.5A Prompt builder with context budget, visibility filtering, secret redaction, and filtered-source audit.
- [ ] Phase 4.5A Agent audit log into Knowledge Store.
- [ ] Phase 4.5A Regression tests for cancellation, approval, tool error, stale ActionRef, provider timeout, secret redaction, architecture context denied, web-query redaction.
- [ ] Phase 4.5A Manual validations for project search, web search, Keynova feature search, private architecture denial, note draft, and terminal/process denial.

## Batch 5 - Plugin And WASM Runtime

- [ ] Phase 4.8 WASM runtime proof-of-concept.
- [ ] Phase 4.8 Host Functions API: log, get_setting, search_workspace, read_clipboard, http_fetch, run_action.
- [ ] Phase 4.8 Plugin action registration.
- [ ] Phase 4.8 Plugin search provider registration.
- [ ] Phase 4.8 Plugin settings schema.
- [ ] Phase 4.8 Plugin audit log into Knowledge Store.
- [ ] Phase 4.8 Hot reload.

## Batch 6 - Future Features And Maintenance

- [ ] Smart Clipboard: classify code/url/path/email/json/error log/command/markdown/image and expose actions.
- [ ] Dev Workflow Pack: git status/branch, cargo test, npm build, open project, search symbol, explain compiler error, create commit message.
- [ ] Command chaining: `/search error | ai explain | note append`.
- [ ] Universal Command Bar over app/file/note/history/command/action with AI fallback.
- [ ] Keep shortcut docs synchronized with `README.md`.
- [ ] Maintain panel first-open/ESC/resize regression checklist.
- [ ] Add provider/action/search source metadata docs for user-facing diagnostics.

## Blocked

- None.
