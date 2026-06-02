---
type: adr
status: proposed
priority: p1
updated: 2026-06-02
context_policy: retrieve_only
owner: project
---

# Remove Asset Protocol; Deliver Image Previews Inline

**Status:** proposed (implemented under explicit developer direction in-conversation; formal acceptance pending)
**Date:** 2026-06-02
**Decision makers:** AI agent draft; developer acceptance required
**Related documents:**
- `docs/security.md`
- `docs/CLAUDE.md`
- `docs/architecture.md`

---

## 1. Context

`tauri.conf.json` enabled the Tauri asset protocol with `assetProtocol.scope = ["**"]`,
and `img-src` in the CSP allowed `asset:` / `https://asset.localhost`. The only
consumer of the asset protocol in the entire frontend is `PreviewPane.tsx`, which
rendered file image previews via `convertFileSrc(preview.path)` — where
`preview.path` is a raw filesystem path returned by the backend `file.preview`
handler (`handlers/file.rs`).

With scope `["**"]`, a compromised renderer (XSS) could `convertFileSrc(<any path>)`
and load arbitrary local files through the asset protocol without even calling a
backend command — an arbitrary local-path read/exfiltration surface. This was the
highest-severity item (#1) in the 2026-06-02 security review. The complementary
review items #2/#4 were closed by `68ff44e`, and #3/#5/#6 by security wave A; #1
remained because narrowing the asset scope in isolation would break image preview
(the path-based load no longer resolves), so it needed a delivery-mechanism change.

Changing the asset/path exposure boundary is a security-boundary change, so per
`docs/CLAUDE.md §3` it is recorded as an ADR. This ADR is `proposed`; the runtime
change is being implemented under the developer's explicit in-conversation
direction (which `docs/CLAUDE.md §4` ranks as the top authority), with formal ADR
acceptance to follow.

## 2. Decision

Deliver image previews inline and remove the asset protocol entirely:

- `file.preview` for `PreviewKind::Image` reads the file (bounded to
  `MAX_INLINE_IMAGE_BYTES = 8 MiB`) and returns a base64 `data:` URL
  (`data:<mime>;base64,...`) instead of a filesystem `path`. Images larger than
  the bound return metadata only with `oversized: true` (no bytes), so IPC and
  renderer memory stay bounded.
- `PreviewPane.tsx` renders `preview.data_url` directly in `<img src>` and drops
  `convertFileSrc`; oversized/missing images show a metadata-only fallback.
- `tauri.conf.json`: `assetProtocol.enable = false` (scope removed), and the CSP
  `img-src` drops `asset:` / `https://asset.localhost`, keeping `'self' data:`
  (already required and sufficient for the inline data URLs).

This removes the renderer's ability to load arbitrary local paths via the asset
protocol. Image bytes are still produced by the backend `file.preview` command,
but only for an explicit, single previewed path and delivered as opaque bytes —
the renderer never receives a path it can re-load.

Rejected alternative: an opaque preview token + custom `kvpreview:` URI scheme
registered in Rust (token → backend-validated path → streamed bytes). It avoids
base64 inflation and supports arbitrarily large images, but adds a protocol
handler, a token store with TTL, and more surface to get right. For a bounded
launcher preview pane the inline data URL is simpler and removes the asset
surface completely; the token approach can be revisited if large-image preview
becomes a requirement.

## 3. Consequences

Positive:
- Eliminates the arbitrary-path asset-load surface (#1); the asset protocol is
  fully disabled and `**` scope is gone.
- No new dependency or custom protocol; reuses the existing `base64` crate and
  the already-present `img-src data:` CSP allowance.

Negative / tradeoffs:
- Image bytes travel over IPC as base64 (~33% inflation), bounded to 8 MiB;
  larger images degrade to metadata-only instead of rendering.
- `file.preview` remains a backend-mediated read for the previewed path (it does
  not yet restrict *which* paths may be previewed beyond existence + classify);
  constraining preview to approved roots is a separate, optional follow-up.

## 4. Rollback

- Re-enable `assetProtocol` (with a *narrowed* scope, not `["**"]`), restore
  `asset:` in the CSP `img-src`, revert `PreviewPane` to `convertFileSrc`, and
  revert the `file.preview` image branch to returning `path`.
- No data-format change to text/binary previews or to non-image flows, so
  rollback is confined to the image-preview delivery path and the config.
