---
type: task_plan
status: active
priority: p1
updated: 2026-06-10
context_policy: on_demand
owner: project
tags: [security, performance, hardening, docs-reconcile]
---

# SEC-PERF: Security + System-Performance Hardening

Branch `hardening/security-perf` (forked from `main`@`ac3986c`). Developer-directed
pass on security + system performance. Scope was pinned after a survey that found
most "security work" is **already implemented** — the real gap is `security.md`
drifting behind REF.8 + the keychain migration. Perf is **measure-first**: memory
(PERF.1.FU) and AI latency (REF.7.D) are already closed, so no blind optimization.

## Survey findings (grounding)

- **API key → OS keychain is ALREADY DONE.** `core/secret_store.rs` (`keyring`
  crate) + `core/config_manager.rs` `set()`/`get()` route `is_sensitive_key`
  values through the OS keychain; `config.toml` only stores a
  `keyring:keynova:<key>` reference, never plaintext. → `security.md` §9 "計劃支援
  OS keychain" row is **stale**.
- **CSP is already tight** (`tauri.conf.json`): `script-src 'self'`, scoped
  `connect-src`, `object-src/frame-src/worker-src 'none'`, `base-uri 'self'`.
  → §9 "CSP 尚未完整設定" row is **stale**. Only residual weakness:
  `style-src 'unsafe-inline'`.
- **Neovim is gone (REF.8).** `security.md` §3.2 (path table `…\nvim\`), §5.1
  (network table "Neovim portable 下載"), §9 ("Neovim 下載 checksum") all
  reference cut features. → **stale**.
- **`npm audit` = 0 vulnerabilities** (frontend deps clean, captured 2026-06-10).

## Primary scope

- [ ] `SEC.1` Dependency vuln baseline. `npm audit` (clean, recorded). Install
  `cargo-audit` + run against `src-tauri` RustSec advisories; record results as a
  point-in-time baseline (Dependabot + CodeQL already cover ongoing). Fix only
  safe, in-range upgrades; majors/breaking → flag, don't force.
- [ ] `SEC.2` `security.md` reconciliation to post-REF.8 + keychain reality.
  Repoint §9 known-limits (keychain DONE, CSP DONE-residual-`unsafe-inline`, drop
  nvim checksum), clean nvim from §3.2 path table + §5.1 network table, add a
  keychain note to §4 sensitive-data handling. Docs-only, no runtime change.
- [x] `SEC.3` `security.md` verification playbook §13–§17 (2026-08-13, docs-only):
  supply-chain (`npm view`) preflight, authorization-bypass regression list,
  XSS/SQLi evidence + rules, agent-config trust boundary (`.mcp.json`/`.claude`/
  `.cursor`/`.amazonq`), SAST/CI gate matrix. Surfaced three new §9 known limits:
  dev origins in production CSP, `npm audit`/`cargo audit` not in CI, and
  `.gitignore` missing the agent-config paths. Detail: `sessions/2026-08-13.md`.
- [~] `SEC.4` Close the three gaps `SEC.3` recorded (2026-08-13):
  - [x] `.github/workflows/audit.yml`: `npm audit --audit-level=high` +
    `npm audit signatures` (advisory) + `cargo audit`, on main/dev push, PRs,
    and a weekly sweep. `cargo-audit` installed from crates.io rather than a
    third-party action (this job *is* the supply-chain gate).
  - [x] `.gitignore` agent-config block (`.mcp.json`, `.cursor*`, `.amazonq/`,
    `.windsurf*`, `.roo/`, `.aider*`, copilot-instructions) — `security.md` §16.3.
  - [x] Dependency remediation the new gate exposed: the 2026-06-10 "0 vulns"
    baseline had rotted to 9 (2 low / 7 high), all dev-toolchain transitives.
    `npm audit fix` resolved all 9 in-range (lockfile only, no `package.json`
    range change); `npm run check` + `vitest` green.
  - [ ] ADR-0057 (`提議`) production CSP split — **needs developer acceptance
    before any `tauri.conf.json` change**. Audit found the renderer makes zero
    direct network calls, so production `connect-src` can drop the dev origins
    *and* all four remote hosts.
  - [ ] Decide on `DEFAULT_NETWORK_ALLOWLIST` dead grants (`api.tavily.com`,
    `duckduckgo.com` — web search removed in REF.8). Default-value change,
    affects existing `security.network_allowlist` overrides; developer call.
  - [x] `cargo audit` half verified locally (2026-08-13, exit 0). Four Rust
    advisories fixed in-range (`quinn-proto`, `serde_with`, `crossbeam-epoch`,
    and `tauri` 2.11.0 → 2.11.1 — CVE-2026-42184 Origin Confusion, the only one
    touching the product's own WebView boundary). `quick-xml` RUSTSEC-2026-0194/
    0195 are unreachable from here (`plist` pins `^0.39.2`) → documented, dated
    ignore in `src-tauri/.cargo/audit.toml`. Running it locally also caught a bug
    in the workflow itself: the `^0.21` pin cannot parse CVSS 4.0 advisories →
    `^0.22`.
  - [ ] CI-side confirmation still pending: `audit.yml` only triggers on
    main/dev push and PRs, so it has not run on this branch.
- [ ] `PERF.1` Measurement baseline (measure-first, NO optimization this batch).
  `cargo tauri build` release; capture cold-start / first-paint, Private-WS
  footprint, and `dist/` bundle sizes. Record baseline in `sessions/2026-06-10.md`.
  Hand the numbers back to the developer to choose an optimization target.

## Nice-to-have / deferred (do not block close-out)

- CSP `style-src 'unsafe-inline'` removal → React inline-style nonce/hash plumbing;
  minor, security-boundary → needs ADR. Defer unless developer prioritizes.
- Confirm `connect-src` `api.anthropic.com` / `api.openai.com` are still reachable
  targets (cloud AI providers); drop if unused.
- Any concrete perf optimization — gated on `PERF.1` baseline numbers.

## Non-goals

- No new security-boundary changes (keychain already done; no new network targets;
  no new file read/write scope) — those would need a fresh ADR per `security.md` §8.
- No `dev`-branch rehab; no deferred refactors (`agent`-named rename, `ai_manager`
  split, DECOUP).

## Validation gates

`cargo test` + `cargo clippy -- -D warnings` + `npm run lint`/`test`/`build` green;
`npm run docs:refresh` guards green (esp. `security.md` schema/size). No behavior
change expected from SEC.2 (docs) or PERF.1 (measurement).
