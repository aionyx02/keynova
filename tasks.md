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
- [x] Agent architecture gap identified: Agent mode still relies on local prompt heuristics, provider chat does not yet accept JSON Schema tools/tool calls, Agent filesystem search still has a bounded DFS fallback, and DuckDuckGo HTML parsing remains fragile.

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

## Batch 4.5 - Agent Quality Recovery

- [x] Direction decision: do not wire an external/third-party agent yet. First harden Keynova's local agent boundary, transparency, approval UX, and regression coverage; add an external-agent provider seam only after this contract feels dependable.
- [x] Agent transparency UI: show planned action kind/risk/payload preview, prompt audit counts, included/filtered grounding sources, tool-call status, memory refs, run error, and delivered UI action.
- [x] Agent convenience first pass: answer capability/time questions directly and label no-action completed runs as answered instead of making them look like silent execution.
- [x] Agent explicit read-only answers: directory-listing prompts now resolve the requested folder, list child folders, or report the checked paths when missing.
- [x] Agent read-only filesystem tools: bounded whole-computer file/folder search plus text preview reads; no write/delete/move capability is exposed.
- [x] Agent web query first pass: `web.search` supports SearXNG or no-key DuckDuckGo fallback, with default provider set to DuckDuckGo.
- [x] Agent workflow planning first pass: task/workflow prompts return a custom execution plan that separates read-only steps from approval-gated side effects.
- [ ] Agent planning quality: make no-action runs explain what Keynova can do next, reduce accidental panel/action matches, and keep high-risk actions scaffolded behind explicit approval.
- [x] Agent planning quality first pass: avoid treating plain "start ..." wording as a terminal command unless the following text looks command-like.
- [ ] Regression tests for cancellation/reject lifecycle, one-shot approval, tool error surfacing, prompt budget truncation, private architecture denial, secret redaction, and web-query redaction.
- [x] Regression test first pass: runtime cancellation/tool registry, terminal command parsing, prompt budget truncation, safe built-in allowlist, private architecture denial, secret redaction, and web-query redaction.
- [ ] External agent integration contract: define the minimum provider interface that can return a plan, tool requests, and Keynova-safe action proposals without bypassing approval/audit.
- [ ] Manual validation script for agent mode: local search, feature search, private architecture denial, note draft approval, terminal approval, rejected approval, disabled web search, and action payload handoff.

## Batch 4.6 - ReAct Tool-Calling Agent Upgrade

- [ ] Write ADR for provider-driven ReAct loop: Rust owns typed tools, policy, approval, audit, and observation handling; the LLM decides tool calls and final answer; local intent heuristics become fallback only.
- [ ] Add provider tool-call contract in `managers/ai_manager.rs`: messages, generated JSON Schema tools, tool calls, tool observations, final text, and unsupported-provider errors; support the selected OpenAI-compatible or Ollama provider first.
- [ ] Agent execution must resolve the same active provider/model as AI Chat (`ai.provider`, Ollama/OpenAI/Claude model settings); no hardcoded OpenAI/API-only path, and local Ollama must remain a first-class Agent backend when selected.
- [x] Add `schemars` and derive tool parameter schemas from Rust structs instead of hand-building JSON; keep OpenAI-compatible tool schema generation covered by snapshot/unit tests.
- [x] Expand `AgentToolSpec` in `core/agent_runtime.rs` with generated parameter schema, result schema, risk, approval policy, visibility policy, timeout, result limits, and observation redaction policy.
- [ ] Implement the async Agent loop in `core/agent_runtime.rs` / `handlers/agent.rs`: build filtered context, inject tool schemas, call provider, execute approved tools, append bounded observations, continue until final text, cancel, timeout, or max steps.
- [x] Add Observation Redaction & Truncation pipeline: hard token/char/line caps, secret redaction, stdout/stderr separation, head+tail preservation with middle redaction marker, and source/result count summaries before context insertion.
- [ ] Move current direct prompt heuristics in `handlers/agent.rs` behind an offline fallback; normal Agent mode should ask the LLM before choosing `keynova.search`, `filesystem.search`, `filesystem.read`, `web.search`, or shell/tool actions.
- [x] Do not ship a generic `execute_shell_command` / `execute_bash_command` tool in the first ReAct pass; replace it with narrow typed tools such as `execute_git_command`, `read_process_info`, and `read_system_logs`, using fixed binaries plus structured args.
- [ ] Write command sandbox ADR before any generic shell tool: no `sh -c` / `cmd /c`, no regex-only read-only policy, platform sandbox requirements, cwd/root limits, network/exfiltration controls, stdout/stderr capture, and denied-error observations for LLM self-correction.
- [ ] Prototype platform command sandbox only after ADR: Linux `bwrap`/namespace/seccomp path, Windows restricted token/job/AppContainer-style constraints, macOS sandbox strategy, and explicit unsupported-platform behavior.
- [ ] Introduce a `SystemIndexer` trait for Agent filesystem tools with capability/status metadata and implementations for Windows Everything/Tantivy, macOS `mdfind`, Linux `plocate`/`locate`, plus bounded parallel `ignore` or `jwalk` fallback.
- [ ] For CLI indexers (`mdfind`, `plocate`, `locate`), use `Command::new` with fixed binary and args, parse stdout with limits, detect missing/stale indexes, and report graceful degradation instead of treating empty results as truth.
- [ ] Add OS search diagnostics: provider availability, index count/age when available, permission-denied count, fallback reason, visited count, timeout, cancellation, and source shown in the Agent UI.
- [ ] Replace DuckDuckGo HTML as the default Agent web path with structured adapters for SearXNG JSON and Tavily API; normalize title/url/snippet/content, trim to token budget, and preserve grounding sources.
- [ ] Keep DuckDuckGo HTML only as explicit best-effort fallback, surfacing parser failures as tool observations instead of pretending the search succeeded.
- [ ] Persist ReAct step audit into Knowledge Store: provider request id, tool schemas offered, tool args preview, approval decision, observation summary, filtered/redacted sources, final answer, and failure/cancel reason.
- [ ] Update the Agent frontend run view for LLM tool use: pending approval, approved/running/completed/denied tool states, stdout preview, observation count, redaction notices, and final grounded answer.
- [ ] Regression tests: provider tool-call parsing, schema generation, observation loop max-step stop, cancellation while waiting/provider/tool running, one-shot approve/reject, unsafe command/tool denial, stdout/stderr truncation, provider timeout, unsupported provider, web-query redaction, stale/missing OS index fallback.
- [ ] Manual validations: find JSON files, read-only file preview, latest web query, denied destructive shell request, typed git status command, project search through Everything/Tantivy/SystemIndexer, stale index warning, rejected approval self-correction, and final answer after tool observation.

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
