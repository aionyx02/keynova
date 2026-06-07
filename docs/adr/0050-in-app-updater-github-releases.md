---
type: adr
status: proposed
priority: p1
updated: 2026-06-06
context_policy: retrieve_only
owner: project
---

# In-App Updater via GitHub Releases (Dormant Until Keyed)

**Status:** proposed
**Date:** 2026-06-06
**Decision makers:** AI agent draft; developer chose the GitHub Releases direction in-conversation
**Related documents:**

- `docs/tasks/product-roadmap.md` (§PRODUCT.3)
- `docs/adr/0048-release-code-signing-pipeline.md`
- `docs/security.md`
- `.github/workflows/release.yml`
- `src-tauri/tauri.conf.json`

---

## 1. Context

PRODUCT.3 (trusted release) wants in-app updates with rollback/failure messaging.
`release.yml` already publishes GitHub Releases via `tauri-action`, so GitHub
Releases is the natural update source. Tauri's updater requires a signing
**keypair** (separate from the code-signing certs in ADR-0048): the public key is
compiled into the app to verify update payloads, and the private key signs the
update artifacts in CI. The developer must generate and hold that keypair
(`tauri signer generate`); the AI must not create/hold a private key, and the
public key cannot be a placeholder (an empty/invalid `pubkey` fails the build).

So the updater is wired now but kept **dormant**: no `plugins.updater` config and
`createUpdaterArtifacts` stays off until the keypair exists, keeping today's build
green and unsigned-update-free.

## 2. Decision

Adopt `tauri-plugin-updater` pointed at GitHub Releases, scaffolded so enabling it
is "generate keypair + add config + add secret", with **zero build/behavior change
until then**:

Wired now (build-safe, in repo):

- `Cargo.toml`: `tauri-plugin-updater` (desktop targets).
- `bootstrap.rs`: plugin registration is **config-gated** — it is added only when
  `plugins.updater` exists in the config (the plugin panics at init without it, so
  unconditional registration would crash `tauri dev`/release). Until configured,
  `/update`'s `check()` fails and the frontend maps it to "not configured".
- `capabilities/default.json`: `updater:default`.
- Frontend `shared/updater.ts` + `/update` command (intercepted in
  `useExecCommand`, like `/onboard`): runs `check()` and renders an inline,
  copy-friendly result — available / up-to-date / not-configured / error. MVP is
  **check-only** ("suggest before execute"); no auto-download/install/relaunch.
- `release.yml`: `TAURI_SIGNING_PRIVATE_KEY` + `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`
  env passthrough (empty → no updater artifacts).

Deferred to the developer (enablement, not in repo):

1. `tauri signer generate` → keep the private key out of the repo.
2. Add to `tauri.conf.json` (this alone flips the config gate, so the plugin
   self-registers — no code edit needed):
   - `bundle.createUpdaterArtifacts: true`
   - `plugins.updater`:
     - `pubkey`: the generated public key
     - `endpoints`:
       `["https://github.com/<owner>/<repo>/releases/latest/download/latest.json"]`
3. Add GitHub Actions secrets `TAURI_SIGNING_PRIVATE_KEY` (+ `_PASSWORD` if set).
4. (Optional follow-up) add `@tauri-apps/plugin-process` + a download/install/
   relaunch action behind explicit user confirmation.

Distribution model: GitHub Releases `latest.json` (generated + uploaded by
`tauri-action` when `createUpdaterArtifacts` + signing key are present). Rejected
alternative: a self-hosted update endpoint — more infra to run and secure for no
benefit while releases already live on GitHub.

## 3. Consequences

- Update capability ships wired; turning it on needs no code change, only the
  keypair + config + secret.
- Until enabled, `/update` honestly reports "not configured"; no network calls.
- The signed update flow (artifact signing, `latest.json`, signature verify,
  install) is **untested until the keypair + a signed release exist** — verify on
  the first keyed release.
- Adds a maintained dependency and one network egress (GitHub Releases) once
  enabled; documented in `security.md` §5.1.
- The updater keypair is distinct from ADR-0048 code-signing certs; both are the
  developer's to hold.

## 4. Rollback

Remove the plugin dep + the config-gated registration + capability, the
`shared/updater.ts` module and the `/update` command/interception, and the CI env
passthrough; drop any `plugins.updater`/`createUpdaterArtifacts` config if added.
No persisted data or schema is involved.
