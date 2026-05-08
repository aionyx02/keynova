# AI Context Privacy

## Visibility Classes

- `public_context`: Safe to include in model prompts and summaries.
- `user_private`: User data that can be used only when the user explicitly asks for it or selects it.
- `private_architecture`: Local project architecture, planning, internal cache/schema/debug content. It may be indexed locally for Keynova UI search, but must not be sent to an external model.
- `secret`: API keys, tokens, credentials, environment secrets, and sensitive settings. Never sent to a model or log.

Unknown sources default to deny.

## Prompt Allowlist

Only these may enter an LLM prompt:

- User's current message.
- `public_context`.
- User-selected `user_private`.
- Redacted summaries whose source visibility is preserved in audit metadata.

## Architecture Denylist

These files and patterns default to `private_architecture`:

- `CLAUDE.md`
- `tasks.md`
- `memory.md`
- `decisions.md`
- `skill.md`
- `docs/*architecture*`
- `files/*架構*`
- internal cache, schema, and debug documents

They may appear in local UI search as redacted results, but not in `/ai` prompt construction or web-search query rewriting.

## Redaction

- Sensitive settings such as API keys are masked in `setting.list_all`.
- Observability logs record payload sizes and timings, not payload contents.
- Filtered sources are audit metadata only and are not sent to a model.

## Web Search Query Policy

`web.search` query rewriting can use only the user's question and allowlisted context. `private_architecture` and `secret` content must be removed before a query leaves the machine.

## Regression Cases

- Secret redaction: API key settings are masked in UI and logs.
- Architecture denied: asking to send architecture docs to the model is refused unless visibility policy changes.
- Web query redaction: local architecture snippets are not included in external search queries.
- Keynova search: private architecture results are shown only as redacted local search hits.

