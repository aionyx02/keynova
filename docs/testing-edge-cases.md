---
type: testing_reference
status: active
priority: p2
updated: 2026-06-02
context_policy: retrieve_when_debugging
owner: project
---

# Testing Edge Cases

Use this file only as structured debugging reference. Put dated root-cause narrative in `docs/memory/sessions/`.

## File Deletion And Mutation

| Scenario                     | Example                     | Expected check                                                                        |
| ---------------------------- | --------------------------- | ------------------------------------------------------------------------------------- |
| Normal file delete           | `C:\...\foo.txt`            | File disappears from source path and row is removed only after backend success.       |
| Folder delete                | Directory with nested files | Recycle-bin operation succeeds or returns actionable error.                           |
| Locked file                  | Open in Notepad or editor   | Backend returns failure or post-mutation verification error; row remains visible.     |
| OneDrive placeholder         | Cloud-only file on Desktop  | Treat as high risk; verify source path after delete and avoid assuming shell success. |
| Network or removable path    | UNC, mapped drive, USB      | Recycle-bin semantics may differ; error text should explain likely cause.             |
| Long path                    | Path over 260 chars         | Canonicalization and mutation should not truncate or normalize incorrectly.           |
| Symlink or junction          | NTFS link or junction       | Mutation should affect the selected path, not silently act on an unexpected target.   |
| Hidden/system/read-only file | `attrib +H/+S/+R`           | Surface permission or shell refusal clearly.                                          |

## Search And Index Staleness

| Scenario                                   | Expected check                                                        |
| ------------------------------------------ | --------------------------------------------------------------------- |
| APP_CACHE returns deleted file             | UI filters missing file/folder paths before display.                  |
| Tantivy or native index returns stale path | Backend post-filters by metadata existence and kind.                  |
| Everything shows Recycle Bin result        | Recently mutated source path stays suppressed for the configured TTL. |
| Workspace switch during search             | Old chunks are discarded or cancelled and selection resets safely.    |
| `:global` query prefix                     | Workspace filter is bypassed only for that query.                     |

## Secondary Action Menu

| Scenario                                 | Expected check                                                      |
| ---------------------------------------- | ------------------------------------------------------------------- |
| First Enter on destructive action        | Shows preview/confirm state; does not mutate.                       |
| Second Enter on same row                 | Executes only if pending action still matches the same result path. |
| Esc after preview                        | Clears pending confirm and inline input.                            |
| Query changes while confirm is pending   | Pending confirm is cleared.                                         |
| Rename or move target exists             | Backend rejects unless explicit overwrite support exists.           |
| Inline path contains separator in rename | Backend rejects unsafe name.                                        |
| Hash on large file                       | Operation stays bounded and user can recover from delay.            |

## Window, Focus, And IME

| Scenario                           | Expected check                                                     |
| ---------------------------------- | ------------------------------------------------------------------ |
| English typing for 30 seconds      | Launcher stays open.                                               |
| CJK IME composition for 30 seconds | Launcher stays open through composition focus blips.               |
| App focus changes briefly          | Auto-hide grace handles transient WebView2 focus changes.          |
| Esc closes launcher                | Grace logic does not prevent explicit close.                       |
| DevTools open                      | Launcher behavior stays debuggable and does not hide unexpectedly. |

## Paths And Text

| Scenario               | Expected check                                                    |
| ---------------------- | ----------------------------------------------------------------- |
| Spaces and punctuation | Shell escaping is not needed for direct filesystem APIs.          |
| Unicode path           | UTF-16/UTF-8 conversion preserves file identity.                  |
| Mixed slash style      | Normalization is deliberate and does not change target semantics. |
| Reserved Windows names | Error is explicit, not a vague filesystem failure.                |
| Trailing whitespace    | Path trimming rules are visible and tested.                       |

## Onboarding, Cheatsheet, And Pipeline

| Scenario                                  | Expected check                                         |
| ----------------------------------------- | ------------------------------------------------------ |
| Onboarding modal plus launcher hotkey     | Listeners do not stack or trap focus.                  |
| `?` cheatsheet while search chunks arrive | Overlay and result state do not collide.               |
| Empty-state CTA uses raw query            | Query text is escaped before note or command creation. |
| Pipeline command with secondary menu open | Menu closes or ignores stale selection safely.         |
