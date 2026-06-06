---
type: adr
status: proposed
priority: p1
updated: 2026-06-06
context_policy: retrieve_only
owner: project
---

# Release Code-Signing Pipeline (Secret-Gated, Skip-When-Absent)

**Status:** proposed
**Date:** 2026-06-06
**Decision makers:** AI agent draft; developer directed the scaffolding in-conversation
**Related documents:**

- `docs/tasks/product-roadmap.md` (§PRODUCT.3)
- `docs/security.md` (§9 known limitations)
- `.github/workflows/release.yml`
- `src-tauri/tauri.conf.json`

---

## 1. Context

PRODUCT.3 (trusted release) requires code signing + notarization so Windows
SmartScreen and macOS Gatekeeper stop warning on first run. `docs/security.md`
§9 already records "未簽署" as a known pending item. Real signing needs
certificates the project does not yet hold:

- Windows: an OV/EV code-signing certificate (CA-issued) or Azure Trusted Signing.
- macOS: an Apple Developer ID certificate + Apple ID app-specific password +
  Team ID for notarization.

These cannot be produced by the AI and must not be committed to the repo (private
keys never enter version control). The developer asked to **wire the pipeline now
and supply certificates later**. A self-signed certificate is explicitly rejected:
it would not satisfy SmartScreen/Gatekeeper and would misrepresent the release as
trusted.

## 2. Decision

Scaffold signing so it activates purely from CI secrets, with **zero behavior
change until secrets exist** (today's build stays unsigned):

- **macOS** — env-driven via `tauri-apps/tauri-action`. The build step receives
  `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`,
  `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID`, `KEYCHAIN_PASSWORD` from
  `secrets.*`. When empty, tauri-action builds unsigned; when set, it imports the
  cert and notarizes. No `tauri.conf.json` change needed.
- **Windows** — `tauri.conf.json` `bundle.windows` gains inert
  `digestAlgorithm: "sha256"` + `timestampUrl` defaults (Tauri only signs when a
  `certificateThumbprint`/`signCommand` is present, so these are no-ops today). A
  **strictly gated** CI step (`matrix.platform == 'windows-latest' &&
  env.WINDOWS_CERTIFICATE != ''`) decodes the base64 PFX from
  `WINDOWS_CERTIFICATE`, imports it, and injects the resolved thumbprint into
  `tauri.conf.json` on the runner. When the secret is absent the step is skipped
  and the config is unchanged.

The private key never lands in the repo; the only artifacts in version control are
the inert signing parameters and the secret-gated CI logic. Updater-artifact
signing (`TAURI_SIGNING_PRIVATE_KEY`) is a **separate** PRODUCT.3 item and is out
of scope here.

Required GitHub Actions secrets (to be added by the developer later):

| Secret | Platform | Purpose |
| --- | --- | --- |
| `WINDOWS_CERTIFICATE` | Windows | base64-encoded PFX |
| `WINDOWS_CERTIFICATE_PASSWORD` | Windows | PFX password |
| `APPLE_CERTIFICATE` | macOS | base64-encoded Developer ID .p12 |
| `APPLE_CERTIFICATE_PASSWORD` | macOS | .p12 password |
| `APPLE_SIGNING_IDENTITY` | macOS | e.g. `Developer ID Application: Name (TEAMID)` |
| `APPLE_ID` | macOS | Apple ID email for notarization |
| `APPLE_PASSWORD` | macOS | app-specific password |
| `APPLE_TEAM_ID` | macOS | Apple Developer Team ID |
| `KEYCHAIN_PASSWORD` | macOS | temp keychain password for tauri-action |

Rejected alternative: commit a self-signed cert / set a hardcoded thumbprint.
Provides no real trust and risks leaking a private key.

## 3. Consequences

- The release workflow is signing-ready; enabling it is "add secrets", no code change.
- Today's tagged releases still produce unsigned bundles — no regression, but the
  SmartScreen/Gatekeeper limitation in security.md §9 stands until secrets land.
- The signed path (esp. the Windows thumbprint injection + notarization) is
  **untested until real certs exist** and must be verified on the first signed run.
- No persisted data, IPC, or runtime behavior changes; only build/release config.

## 4. Rollback

Remove the `env:` passthrough + the Windows signing prep step from
`release.yml`, and drop the `digestAlgorithm`/`timestampUrl` keys from
`tauri.conf.json`. No data migration is involved.
