# Plugin Security Model

## Threat Model

Plugins are untrusted by default. A plugin may be buggy, malicious, abandoned, or compromised through its distribution channel. Keynova must assume plugins can attempt to exfiltrate secrets, run expensive loops, call host functions repeatedly, or confuse users with unsafe actions.

## Trust Boundary

The boundary is between plugin bytecode and Keynova host functions. Plugins do not receive direct WebView eval, unrestricted JavaScript, shell access, filesystem access, clipboard access, AI keys, or system control.

## Runtime Direction

Keynova uses a WASM-first plugin runtime. JavaScript plugins are deferred and must use the same capability-based host ops if introduced later.

## Permission Model

Allowed capabilities are explicit manifest entries:

- `log`
- `get_setting`
- `search_workspace`
- `read_clipboard`
- `http_fetch`
- `run_action`

Denied by default:

- filesystem
- shell
- network beyond declared `http_fetch` targets
- clipboard write
- AI key access
- system control

## Host Functions

All plugin abilities go through bounded host functions: `log`, `get_setting`, `search_workspace`, `read_clipboard`, `http_fetch`, and `run_action`. Every host call is permission-checked and audit logged.

## Resource Limits

Each plugin execution has limits for memory, execution time, host-call count, output size, and search result count. Plugin search providers must also respect timeout, cancellation, and result limits.

## Crash Isolation

A plugin crash must fail the plugin execution and emit an audit entry. It must not crash the main process or block federated search.

## Current Implementation

Phase 4 adds manifest types, default resource limits, permission checks, audit entries, and validation that rejects dangerous permissions until an explicit grant flow exists.

