---
type: coding_style
status: active
priority: p1
updated: 2026-06-02
context_policy: retrieve_when_debugging
owner: project
---

# Coding style

Keynova follows Google style where Google publishes a guide for the language,
and uses the Rust default style for Rust code. The local tools are the source of
truth for formatting, so style disagreements should usually be resolved by
running the formatter instead of hand-tuning whitespace.

Authoritative references:

- [Google TypeScript Style Guide](https://google.github.io/styleguide/tsguide.html)
- [Google HTML/CSS Style Guide](https://google.github.io/styleguide/htmlcssguide.html)
- [Google Markdown Style Guide](https://google.github.io/styleguide/docguide/style.html)
- [The Rust Style Guide](https://doc.rust-lang.org/nightly/style-guide/)

## Required checks

Run the focused checks for the area you touched before review:

```bash
npm run format:check
npm run lint
npm run build
npm run test
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

Use `npm run verify` before larger changes or before release work.

## Repository-wide rules

- Keep source files in UTF-8 without BOM. Do not leave mojibake, replacement
  characters, or comments whose encoding is broken.
- Prefer boring, explicit code over clever shortcuts. Local helpers and existing
  module boundaries win over new abstractions.
- Comments explain intent, constraints, and security boundaries. Remove comments
  that only repeat the code.
- Use `TODO: ...` for action items. Include the concrete next action, not a
  vague marker.
- Keep unrelated refactors out of feature changes. If cleanup is useful, make it
  a small dedicated change.
- Treat shelling out, filesystem traversal, IPC, network access, and secrets as
  security-sensitive code paths. Validate inputs at the boundary.

## TypeScript and React

- Keep `strict` TypeScript green. Avoid `any`; prefer typed payloads, `unknown`
  plus narrowing, or a small parser at the boundary.
- Use named exports for shared modules. Default exports are allowed only when the
  framework or tooling entrypoint requires them.
- Prefer `const`; use `let` only when a binding is reassigned. Do not use `var`.
- Use `import type` and `export type` for type-only symbols.
- Keep React components focused on rendering. Put state transitions, effects, and
  IO behind hooks or feature-local utilities.
- Use clear names: `camelCase` for values and functions, `PascalCase` for React
  components and types, and `UPPER_SNAKE_CASE` only for stable constants.
- Do not build large stringly typed command payloads inline. Define typed request
  and response shapes near the boundary.
- Keep user-visible strings together with the feature that owns them so later
  localization remains possible.

## CSS and UI markup

- Use 2-space indentation, lowercase HTML/CSS syntax, and semantic markup.
- Prefer class selectors over ID selectors. Class names should describe purpose,
  not color or one-off placement.
- Use hyphen-separated CSS class names.
- Prefer valid HTML and accessible controls. Interactive elements should be real
  buttons, inputs, anchors, or controls with equivalent semantics.
- Use CSS shorthand when it improves readability, and omit units on zero values
  unless the browser behavior requires a unit.

## Rust

- Use `cargo fmt` defaults. Rust source uses 4-space indentation and a 100-column
  target width.
- Keep modules small and behavior-oriented. Put platform-specific code under the
  platform module instead of scattering `cfg` blocks through feature managers.
- Return typed errors for shared APIs when callers need to branch on the cause.
  `String` errors are acceptable for narrow command boundaries that only display
  a message.
- Avoid holding locks while doing IO, walking the filesystem, launching
  processes, or publishing events.
- Prefer structured Windows, Tauri, or Rust APIs over shell commands. If a shell
  is unavoidable, validate arguments and never compose commands from user input.
- Keep `unsafe` localized, documented, and justified by the API contract it wraps.
- Tests for security boundaries should assert both the accepted path and the
  rejected path.

## Markdown and docs

- Use one H1 per document, then H2/H3 headings.
- Keep headings complete and unique enough to be useful as anchors.
- Prefer short paragraphs and explicit links. Use fenced code blocks with a
  language tag when possible.
- Keep docs current with code changes. If a change modifies behavior, update the
  smallest relevant doc instead of adding duplicate notes elsewhere.
