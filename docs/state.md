# State

Where the project actually is, what is unfinished, and what is true outside this
repo. Updated when one of those changes — not per task, not per session.

Last touched: 2026-08-23

## Now

`v0.7.1`, in maintenance. The last feature work landed 2026-06-14; everything
since has been security, dependency and CI work. Keynova is a keyboard-first
launcher (Tauri 2 + React + Rust); AI is an inline capability, not the core.

## Branching

`feature/*` or `chore/*` off `dev` → PR → `dev` → PR → `main`. `main` is always
releasable.

Both are protected: 8 required checks (`rust` ×3, `Analyze` ×2, `npm audit`,
`cargo audit`, `frontend`), PR required, **no approving review required** —
GitHub forbids approving your own PR, so requiring one deadlocks a solo repo.
Merging is the review. Admins are not enforced, so there is an escape hatch.

`dev` died once, falling 191 commits behind because work forked from `main`
while the old rule forbade merging `main` back. Rebuilt at `main` 2026-08-23,
and **`main` → `dev` sync is now allowed** — without it `dev` rots again.

## Toolchain

Rust is pinned in `rust-toolchain.toml` — one number, read by CI and by a
developer machine alike. Upgrading is bumping `channel` and letting CI say what
broke.

`toolchain-drift.yml` runs clippy and the tests against the latest stable every
Monday. It is informational and not a required check: a red run means the next
bump has work waiting, not that anything is broken now. Without it, pinning
would trade random CI failures for silence.

## Unfinished

- **Releases are unsigned.** `release.yml` builds a draft on three OSes with
  secret-gated signing steps that stay inert until the ADR-0048 Windows/Apple
  certificate secrets are added. SmartScreen and Gatekeeper warn.
- **`XPLAT` phase 2**: `platform/linux.rs` and `macos.rs` are skeletons. Phase 1
  (3-OS CI, no panics) is done.
- **`嚙` mojibake root cause** — masked, not solved. See Traps in `decisions.md`.
- **`ICON.NATIVE` + UX** and the `SEC-PERF` perf baseline are mid-flight.

## Security gaps (2026-08-13 audit)

Accepted for now, not oversights:

- No `cargo-deny`; only RustSec advisories block.
- `npm audit signatures` is `continue-on-error` — a missing signature is
  usually a registry gap.
- CodeQL analyses Rust with `build-mode: none`, so coverage is shallow.
- No SBOM; release artifacts unsigned; secret scanning not configured.
- `style-src 'unsafe-inline'` remains — removing it needs nonce/hash plumbing
  for React inline styles.
- `DEFAULT_NETWORK_ALLOWLIST` still lists `api.tavily.com` and
  `duckduckgo.com` though REF.8 removed the web-search provider. Dropping them
  changes what existing `security.network_allowlist` overrides mean, so it is a
  developer decision.

## Frozen

Not blocked on anything — deliberately not being worked on: `AGENT.*`, `CLIP.1`,
`SNIP.1`, `WIN.1`, `UTIL.3`, `DEV.1`, `SYNC.1`, `LAUNCH.2.C`, `ONBOARD.1.D/E`,
`NOTE.1`, `UTIL.1.B-online`.

Two things are genuinely blocked and must not be started:

- **Generic shell execution.** The sandbox boundary is incomplete; an approval
  flow is not sufficient. Deterministic typed tools are fine.
- **`FEAT.11` runtime scanning of private files.** ADR-0028 set a local-context
  security boundary that is not yet met. Denylist/redaction unit tests are fine.

## Outside this repo

- Dependabot retired 2026-08-23: 21 branches and PRs `#4`–`#34` closed,
  `.github/dependabot.yml` deleted. Dependency updates are manual
  (`npm outdated` / `cargo update`, `npm audit` / `cargo audit`). CodeQL is
  unaffected.
- The `gh` token needs the `workflow` scope to push changes under
  `.github/workflows/`.
- There are no git hooks. Checks run in CI, which is the only place they have
  ever run reliably; the old pre-commit hook installed itself through `npm ci`
  and so had never executed on the developer's machine at all.
