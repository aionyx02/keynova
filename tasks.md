# Keynova Tasks

> Compact task board. Keep this file short: completed work is summarized, remaining work is grouped into executable batches.
> Update `memory.md` and `decisions.md` when a batch changes architecture or runtime behavior.

## Current Baseline

- [x] Phase 0-3.8 foundation: Tauri/React/Rust scaffold, CommandRouter/EventBus, launcher UI, app/file search, terminal panel, settings, hotkeys, mouse/system controls, AI/translation/workspace/note/calculator/history/model features.
- [x] BUG-1 to BUG-15 user-facing fixes, including first-open lazy panel sizing/scrollbar refresh.
- [x] Phase 2 search backend debug/config/rebuild/test path, with `tantivy` preference currently backed by the rebuilt offline file cache; persisted Tantivy index still pending.
- [x] Phase 4 foundation: ActionRef/ActionArena, UiSearchItem, action.run, secondary actions, schema-driven settings, Workspace Context v2 base, Knowledge Store DB worker, Agent mode skeleton, Automation/Plugin security docs.
- [x] Knowledge Store batch writes, bounded queue/drop policy, shutdown flush, DB observability.
- [x] Plugin manifest loader and deny-by-default permission parser.
- [x] Agent read-only tool runtime: `agent.tool`, `keynova.search`, SearXNG-backed `web.search`, visibility filtering, grounding sources, search cancel/generation checks.

Last full verification baseline: `npm run build`, `npm run lint`, `cargo test`, `cargo clippy -- -D warnings`, `git diff --check`.

## Batch 1 - Automation And App Structure

- [x] Phase 4.6 Action chain executor.
- [x] Phase 4.0 Split `src-tauri/src/lib.rs` into `src-tauri/src/app/*` modules (`state.rs`, `bootstrap.rs`, `shortcuts.rs`, `tray.rs`, `window.rs`, `watchers.rs`, `control_server.rs`).

## Batch 2 - Search Runtime

- [x] Phase 4.1 Search chunked/progressive streaming so first visible batch returns under 50ms.
- [x] Phase 4.1 First batch cap/paging: return first 10-20 items, then page/lazy fetch up to 50+.
- [x] Phase 4.1 Preview and metadata lazy load through `search.metadata`.
- [x] Phase 4.1 Icon asset lazy load beyond lightweight `icon_key`.
- [x] Phase 4.4 Progressive results over Tauri channel chunks.
- [x] Phase 4.4 Provider timeout boundary for slow file/search providers.
- [x] Phase 2/4.4 Offline file provider path for the `tantivy` backend preference using the existing rebuilt file cache.
- [x] Phase 2/4.4 Persisted Tantivy index with real Tantivy schema and on-disk index storage.

## Batch 3 - Workspace And Knowledge

- [x] Phase 4.3 Bind terminal sessions to current workspace.
- [x] Phase 4.3 Bind notes to current workspace.
- [x] Phase 4.3 Bind AI conversations to current workspace.
- [x] Phase 4.3 Workspace-weighted clipboard/history ranking.
- [x] Phase 4.5 Recent/frequent search ranking memory cache.
- [x] Phase 4.5 Migration and backup path for Knowledge Store.

## Batch 4 - Agent Runtime

- [x] Phase 4.5A Approval state machine and explicit agent action boundary: pending approvals, action allowlists, per-risk execution policy.
- [x] Phase 4.5A Prompt builder with context budget, visibility filtering, secret redaction, and filtered-source audit.
- [x] Phase 4.5A Medium-risk approved actions: open panel, create note draft, update setting draft, run allowlisted safe built-in command.
- [x] Phase 4.5A High-risk action scaffolding with explicit approval: terminal command, file write, system control, model download/delete.
- [x] Phase 4.5A Agent audit log into Knowledge Store.
- [x] Phase 4.5A Agent memory layers: session, workspace, long-term with opt-in.
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

### /note + LazyVim Workflow (Feasibility Confirmed)
- [x] Extend CommandUiType with Terminal(TerminalLaunchSpec); keep Inline/Panel unchanged.
- [x] Update frontend BuiltinCommandResult type to support Terminal result.
- [x] Change NoteCommand to parse:
  /note
  /note lazyvim
  /note lazyvim [note-name]
  /note lazyvim --path <absolute-or-relative-path>
- [x] Inject NoteManager/config into NoteCommand, or route lazyvim branch through NoteHandler.
- [x] Expose NoteManager path helpers:
  notes_root()
  resolve_named_note(name) -> absolute .md path
  create_parent_dirs_for_file(path)
- [x] Add nvim availability detection:
  configured command -> PATH lookup -> fallback result.
- [x] Add TerminalManager.create_pty_with_command(...), bypassing shell.
- [x] Update terminal.open payload to accept launch spec.
- [x] Update TerminalPanel to recreate PTY when launchId changes.
- [x] Disable Escape-to-exit for LazyVim/editor sessions; use Ctrl+Alt+Esc or Ctrl+Shift+Q instead.
- [x] Keep /note default behavior returning Panel("note").
- [x] Add fallback behavior:
  nvim missing -> Inline guidance, optionally "open builtin note editor" action later.
- [x] Detect official LazyVim starter config and include LazyVim/LazyVim + LazyVim/starter guidance.
- [x] Support notes.lazyvim_config_dir and terminal env injection for NVIM_APPNAME/XDG_CONFIG_HOME.
- [x] Auto-bootstrap missing LazyVim starter config into project-local `.keynova/lazyvim/`.
- [x] Keep LazyVim config/data/state/cache project-contained and git-ignored.
- [x] Add tests for:
  /note no args
  /note lazyvim
  /note lazyvim foo
  /note lazyvim --path ...
  path sanitization
  nvim missing
  LazyVim config missing
  Terminal launch result serialization
- [x] Verify automated:
  npm run lint
  npm run build
  cargo test
- [ ] Manual verify:
  /note, /note lazyvim, /note lazyvim foo, Esc inside Vim, terminal exit shortcut


## Blocked

- None.
