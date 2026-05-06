# Keynova Regression Checklist

Use this checklist before and after Phase 4 refactors. The goal is to catch behavior drift early while the Action System, search boundary, and runtime internals are being reshaped.

## Environment

- OS: Windows 11
- Build mode: `npm run tauri dev` for behavior checks, `npm run tauri build` for package checks
- Config file: `%APPDATA%\Keynova\config.toml`
- Notes: keep DevTools Console open during checks and record any IPC/CSP errors

## Manual Checks

### Search

- [x] Type an app name and confirm app results appear within the normal debounce window.
- [x] Type a known file/folder name and confirm file/folder results appear when the active backend supports it.
- [x] Press `ArrowDown`/`ArrowUp`; selection moves without the input bar scrolling away.
- [x] Press `Enter` on an app result; the app launches and the launcher hides.
- [x] Confirm `search.backend` still reports the expected backend through the command path used by the UI.

### Terminal

- [x] Type `>` to enter terminal mode.
- [x] Run `ipconfig`.
- [x] Press physical `Esc`; the launcher returns to search mode and does not hide.
- [x] Re-enter `>`; the terminal session remains mounted and previous output is still visible.

### Command

- [x] Type `/command` or `/help`; command suggestions render.
- [x] At the first suggestion, press `ArrowUp`; focus visually returns to the input row.
- [ ] Press `Tab` on a command suggestion; the command fills into the input.
- [x] Execute a panel command and press `Esc` inside the panel; the panel closes and focus returns to input.

### Setting

- [x] Run `/setting launcher.max_results 12`; value persists.
- [x] Confirm config reload emits a reload event and search limit updates without restarting.
- [x] Run `/setting` and verify table keyboard navigation still works.
- [ ] Restart Keynova and confirm changed settings are loaded.

### Hotkey

- [x] Press launcher hotkey (`Ctrl+K` by default); launcher toggles visibility.
- [x] Press mouse-control hotkey (`Ctrl+Alt+M` by default); overlay state toggles.
- [ ] Call `hotkey.register`; it returns a clear not-implemented error, not a panic or silent failure.

### Mouse

- [x] Enable mouse control.
- [x] Press `Ctrl+Alt+W/A/S/D`; cursor moves by the configured step.
- [x] Press `Ctrl+Alt+Enter`; left click is sent.
- [x] Disable mouse control and confirm movement/click hotkeys no longer act.

### AI

- [x] Open `/ai`; configured provider is displayed.
- [ ] Send a short prompt and receive a response or a readable provider error.
- [ ] Confirm API keys are not printed in DevTools logs or backend debug output.

### Translation

- [ ] Run `/tr en zh-TW hello`; result is `你好`.
- [ ] Disconnect network or trigger a bad request; the panel shows a readable Google Translate error.
- [ ] Confirm `/tr` no longer offers Ollama, Claude, or OpenAI-compatible model selection.

### Model

- [ ] Open `/model_list`; only AI Chat models/providers are listed.
- [ ] Switch the active AI Chat model; the selected model persists.
- [ ] Open `/model_download`; RAM/VRAM recommendation is visible.
- [ ] Start a small model download or mock-safe equivalent; progress/done/error events render.

### Workspace

- [ ] Press `Ctrl+Alt+1/2/3`; workspace indicator switches.
- [ ] Edit a workspace query, switch away, then switch back; query is restored.
- [ ] Run note/history commands after switching workspace; existing behavior remains stable.

## Observability Baseline

Record these values whenever a Phase 4 milestone changes IPC, search, startup, or persistence behavior.

- [ ] IPC command latency: inspect `[keynova][obs] ipc.command` debug logs during Search, Command, Setting, and Model flows.
- [ ] Search latency: inspect `[keynova][obs] search.query` debug logs while typing a 3-5 character query.
- [ ] Action-like command latency: inspect `[keynova][obs] action.execution` debug logs for `/help`, `/setting`, `/model_list`.
- [ ] Idle memory: capture Keynova process working set after 60 seconds idle.
- [ ] Idle CPU: capture Keynova process CPU after 60 seconds idle.
- [ ] Production build size: run `npm run tauri build` and record installer/bundle size.

## Pass Criteria

- No user-visible regression in checked flows.
- No DevTools Console error except expected provider/network failures.
- Structured IPC errors include `code` and `message`.
- Debug observability logs avoid sensitive text values and API keys.
