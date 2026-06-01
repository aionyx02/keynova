---
type: adr
status: proposed
priority: p1
updated: 2026-06-01
context_policy: retrieve_only
owner: project
---

# Secret At-Rest Storage via OS Keychain

**Status:** proposed
**Date:** 2026-06-01
**Decision makers:** AI agent draft; developer acceptance required
**Related documents:**
- `docs/security.md`
- `docs/adr/0010-config-manager-toml.md`
- `docs/CLAUDE.md`

---

## 1. Context

All four secret settings — `ai.api_key`, `ai.openai_api_key`,
`agent.web_search_api_key`, `translation.api_key` — are persisted in cleartext
in `%APPDATA%\Keynova\config.toml` by `ConfigManager::persist()`
(`src-tauri/src/core/config_manager.rs`). `docs/security.md §9` already lists
this as a known limitation with a planned move to an OS keychain, and §4.3 names
config secret fields as data that should not sit in plaintext.

Recent work already closed the *display/transport* surface: IPC responses redact
secrets (`setting.get` → `********`, `list_all_redacted()` empties values), the
settings/model UI uses `type="password"`, workflow-history labels are redacted,
payloads are hashed, and logs emit metrics only. The renderer no longer retains
plaintext after a save either (the `SettingPanel.saveValue` fix). What remains is
the **at-rest** exposure: anyone with filesystem access (backups, sync, restore
points, another local process) can read the keys directly from `config.toml`.

Changing where secrets live is a security-boundary change, so per
`docs/security.md §8` and `docs/CLAUDE.md §3` it requires an accepted ADR before
runtime implementation. This ADR is `proposed` only; no runtime change ships
until the developer accepts it.

## 2. Decision

If accepted, Keynova will store the four secret keys in the OS credential store
instead of `config.toml`:

- Windows Credential Manager, macOS Keychain, Linux secret-service, via the
  `keyring` crate (cross-platform, no new daemon dependency).
- `ConfigManager` gains a thin indirection for sensitive keys (those for which
  `is_sensitive_key()` is true): reads/writes route to the credential store
  while non-secret keys stay in `config.toml`. The existing
  `is_sensitive_key` / `redact_setting_value` contract in
  `models/settings_schema.rs` is the single source of truth for which keys are
  secret, so no second list is introduced.
- One-time migration: on first run after upgrade, any plaintext secret found in
  `config.toml` is moved into the credential store and erased from the TOML file
  (the key is left absent, not blanked, to avoid re-persisting an empty marker).
- Graceful fallback: when the OS store is unavailable (headless Linux without
  secret-service, locked keyring), the secret is treated as unset and the
  relevant feature surfaces its existing "API key not set" path rather than
  silently falling back to plaintext on disk.
- `docs/security.md §4.3` and `§9` are updated to record the credential store as
  the canonical secret location.

Rejected alternative: encrypting the secret fields inside `config.toml` with a
machine-bound key. It keeps ciphertext on disk next to a derivable key, offering
weaker guarantees than the OS store while still requiring key-management code.

## 3. Consequences

Positive:

- Secrets leave the plaintext config file; at-rest exposure drops to whatever
  the OS credential store protects.
- Reuses the existing sensitive-key contract, so the schema stays the one place
  that defines what is secret.
- Closes the last open item in `security.md §9` for API-key storage.

Negative / tradeoffs:

- Adds the `keyring` dependency and platform-specific backends to test
  (Windows / macOS / Linux secret-service), including the no-backend case.
- Migration touches existing user config; it must be idempotent and must not
  destroy a key it failed to store.
- Settings/model panels need to read "is a secret configured?" from the
  credential store rather than inferring it from a non-empty TOML value
  (`model.rs` `configured:` flags, `check_setup` missing-key logic).

## 4. Rollback

- Revert the `ConfigManager` indirection so sensitive keys read/write
  `config.toml` again; remove the `keyring` dependency.
- Provide a reverse migration (or accept manual re-entry): on rollback, secrets
  living only in the credential store are treated as unset and the user re-adds
  them via `/setting`, which writes back to `config.toml` under the old path.
- No schema or data-format change to non-secret settings, so rollback is limited
  to the secret-storage layer and the migration step.
