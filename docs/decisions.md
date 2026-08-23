# Decisions and Traps

Why parts of Keynova are the way they are, and what has bitten us. Nothing here
is derivable from the code — module layout, IPC routes, event topics and config
shapes are not documented because `grep` answers those correctly and this file
would not.

`ADR-NNNN` numbers appear in source comments. The full text of every ADR lives
in git history (`git log --diff-filter=D -- docs/adr/`); the entries below are
what a reader needs without opening it.

## Why

**ADR-0029 — AI is a capability layer, not the product.** AI moved out of the
core: no chat-first panel, no autonomous ReAct loop. Every call is a stateless,
single-shot `call_capability`. A capability may not call another capability;
chaining is allowed only when the user drives it by triggering successive
`ActionChip`s. A chip must be disabled while its own call is in flight —
state-based, not a time debounce. The legacy agent was physically deleted in
REF.8 (2026-06-09), superseding ADRs 0016/0022/0023/0026.

**ADR-0030 — Risk tags are minimal and fail safe.** Backend returns
`{requires_confirmation: bool, reason: String}` and nothing else; confirmation
is owned by the UI. Deliberately no category × severity matrix. A missing
field, an unparseable tag, or an absent `requires_confirmation` is treated as
`true` — the failure mode must be "ask the user", never "just do it". `reason`
goes to the audit log only, is never rendered, and is capped at 256 chars.

**ADR-0043 — Personal memory reuses the existing table.** `remember` / `recall`
store into `agent_memories` with `scope="personal"`, `visibility="private"`. No
new table, no migration. `recall` uses no LLM at all — local read plus term
ranking. Memory grounding is gated on a local provider only.

**ADR-0044 — Features register themselves.** Each feature exposes
`register(&mut FeatureRegistrar, &AssemblyCtx)` and wires its own handlers,
builtins, search hooks and settings fragments. Central assembly iterates a
manifest instead of hand-listing features, so removing a feature is deleting a
module plus its manifest line. `AssemblyCtx` carries only genuinely shared
infrastructure; a leaf feature builds its own private manager.

**ADR-0045 — The interactive terminal is human-only.** `terminal.request_shell`
is reachable only by an explicit user gesture (`>` in the focused palette). It
builds and registers a one-time `TerminalLaunchSpec`, so every PTY is still
backend-issued and single-use. Non-human actors get no shell.

**ADR-0047 — `/diag` is metadata, never contents.** The bundle carries app
facts, feature flags, redacted config, and presence plus on-disk size of local
data. It never reads file contents. Redaction is contractual, not best-effort.

**ADR-0050 — The updater is dormant until keyed.** `tauri-plugin-updater`
registration is config-gated on `plugins.updater` existing. This is not
tidiness: the plugin panics at init without config, so unconditional
registration crashes `tauri dev`. Until configured, `/update` fails and the
frontend reports "not configured".

**ADR-0051 — Panic hook is best-effort and chains.** A process-wide hook
installed early in `bootstrap::run` appends one redacted line per panic to
`%LOCALAPPDATA%\Keynova\crash.log`, then chains to the previous hook so stderr
behaviour is unchanged. Home paths are collapsed, messages length-capped, write
failures swallowed. The hook must never itself panic.

**ADR-0052 / ADR-0053 / ADR-0054 — Ranking signals, added additively.**
Transition frequency `P(next | anchor)` blended with recency (0052); a nullable
`succeeded` column recording both `Ok` and `Err` outcomes (0053); a nullable
`project_root` column keying workspace profiles (0054). Schema v4 → v7 each went
in as an idempotent `ALTER TABLE ... ADD COLUMN` guarded by `PRAGMA table_info`,
so old rows stay valid and rollback is the existing pre-migration backup. Thin
transition support falls back to recency — cold start never regresses.

**ADR-0056 — Native Win32 icon extraction, not PowerShell.** Removes a runtime
`powershell.exe` dependency and its spawn cost. Chose FFI over GDI+ (no global
lifecycle risk) and added `png` (pure Rust, small) rather than `image` (too
heavy for the footprint budget). Roughly 200 lines of Windows-only unsafe FFI.
The on-disk `.b64` cache format is unchanged, so old entries still work.

**ADR-0057 — Production CSP carries no dev origins.** `security.csp` is
`connect-src 'self' ipc: http://ipc.localhost`; the two dev origins and four
remote hosts live only in `security.devCsp`, which Tauri injects for `tauri dev`
alone. Rejected the alternative of rewriting the CSP string at build time —
`devCsp` is the official mechanism and a custom build step would drift.

**Release profile is deliberately conservative.** `[profile.release]` uses strip
plus LTO but explicitly not `opt-level = "z"` and not `panic = "abort"` —
abort would defeat the ADR-0051 crash log.

## Traps

**Every Windows `Command` spawn must chain `.no_window()`.**
`core::SilentCommandExt` sets `CREATE_NO_WINDOW` on Windows and is a no-op
elsewhere. Miss one and a console window flashes on screen and steals focus from
the launcher. This is not cosmetic — focus theft closes the palette.

**Do not rename the CLI bin or `productName`.** There are two cargo bins:
`tauri-app` (GUI, `default-run`) and `keynova` (CLI). Tauri matches
`productName` "Keynova" against bin targets case-insensitively, picked the wrong
one, and broke macOS universal bundling. The fix is
`"mainBinaryName": "Keynova"` in `tauri.conf.json`. Changing either name
re-opens the collision.

**EventBus topics are dotted; Tauri topics are dashed.** `terminal.output` is
emitted to the frontend as `terminal-output` via
`AppEvent::legacy_tauri_topic()`. Subscribing with the dotted name silently
receives nothing.

**Search runs at most one background task.** `SearchService` is a single worker
with a Condvar slot: one pending, one running. Submitting a new task cancels the
previous through an `Arc<AtomicBool>`, and file search has a hard 800 ms
timeout. Fast typing must cancel, not queue — adding parallelism here reopens
PERF.2.

**`嚙` mojibake has no known root cause.**
`SearchResultsList.hasEncodingError('嚙')` masks it in the UI. That is a
documented fallback, not a fix; no live read path has ever been shown to produce
it. Treat a sighting as a new data point, not a regression of a solved bug.

**Local and CI must measure with the same ruler.** Two failures of this shape
have already cost a red pipeline. `core.autocrlf=true` means a checkout adds one
byte per line on Windows, so byte-size checks must normalize to LF first. And
CI resolves `dtolnay/rust-toolchain@stable` while a developer machine sits on
whatever it last installed — Rust 1.98 added
`clippy::chunks_exact_to_as_chunks`, which fired on five `cfg(windows)` sites
while local clippy (1.95) reported clean. Nothing pins the toolchain; a new
stable release can turn CI red with zero code change.

**Never hold a lock across IO.** Not across a filesystem walk, a process
launch, or an EventBus publish either. The search worker and the knowledge-store
actor both exist partly because of this.

**A security-boundary test asserts the rejection too.** Testing that the allowed
path works proves nothing about the boundary. Both directions, always.

**Keep `unsafe` localized and justified by the API contract it wraps.** ADR-0056
added ~200 lines of Windows FFI; that is the ceiling, not a precedent.

**Source files are UTF-8 without BOM.** No mojibake, no replacement characters,
no comments with broken encoding — see the `嚙` trap for why this is not
pedantry here.

Everything else about style is enforced by `prettier`, `eslint`, `cargo fmt` and
`clippy`, or is generic Google/Rust style. Run the formatter instead of arguing.

**Memory numbers: use Private Working Set.** Real unique footprint is about
80 MB. The ~324 MB process-tree figure is shared Edge/Chromium DLL pages, not
Keynova's. The "Background Core" target excludes the active WebView, loaded LLM
model memory, PTY sessions, monitoring streams and index rebuilds.
