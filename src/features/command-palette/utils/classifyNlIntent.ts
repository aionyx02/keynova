// Rule-based NL intent classifier for the smart capability fallback.
//
// Layered on top of `looksLikeAiCommandIntent` (the existing `cmd` heuristic).
// Runs only in the "no search results + stabilization debounce" fallback path
// in CommandPalette: when the user types something that did not match any
// search result or builtin command, classify it into one of the 5 capability
// surfaces so they can act on it without typing the explicit prefix.
//
// Decision order (first match wins):
//   1. Error-shaped input (cargo/npm error, panic, stack trace) -> fix
//   2. Imperative "fix"/"修復" verbs -> fix
//   3. Summarize cues ("summarize", "tldr", "總結", ...) -> summarize
//   4. Explain cues ("what is", "explain", "解釋", ...) -> explain
//   5. Action-verb heuristic delegated to looksLikeAiCommandIntent -> cmd
//   6. null (no smart surface)
//
// `fix` is checked before `explain` so "fix this error" wins over "what error".
// `cmd` is the most permissive bucket and runs last.
//
// Returns the original trimmed query as `text` (no leading-verb stripping).
// Reason: every backend capability either re-wraps the text in its own system
// prompt (explain/summarize/fix) or treats it as a free-form intent (cmd),
// so the extra verb is harmless and keeps mental-model 1:1 with what the
// user sees.

import { looksLikeAiCommandIntent } from "./looksLikeAiCommandIntent";

export type NlIntentId = "explain" | "summarize" | "fix" | "cmd";

export interface NlIntentMatch {
  id: NlIntentId;
  text: string;
}

const FIX_SHAPED = [
  /\berror\[[A-Z]\d+\]/i, // cargo error code: error[E0308]
  /\bpanicked at\b/i,
  /\bstack trace\b/i,
  /\btraceback\b/i,
  /\bexit(?:ed)? with (?:status |code )?\d+/i,
  /\b(?:cannot|can't|could not) find\b/i,
  /\bsegmentation fault\b/i,
  /\bunhandled exception\b/i,
  /\bsyntaxerror\b/i,
  /\btypeerror\b/i,
  /\breferenceerror\b/i,
];

const FIX_VERB_EN = [
  /^fix\b/i,
  /^debug\b/i,
  /^why\s+(?:does|is|did|are|am)\b.*\b(?:fail|error|crash|broken|wrong)\b/i,
  /^what(?:'s|\s+is)\s+wrong\b/i,
];

const SUMMARIZE_VERB_EN = [
  /^summari[sz]e\b/i,
  /^summary(?:\s+of|:)\b/i,
  /^sum\s+up\b/i,
  /^tl;?dr\b/i,
];

const EXPLAIN_VERB_EN = [
  /^explain\b/i,
  /^what\s+(?:is|are|does|do|'s)\b/i,
  /^how\s+(?:does|do|is|are)\b.*\bwork\b/i,
  /^why\s+(?:does|do|is|are)\b/i, // generic "why does X"; fix branch above catches error-flavored ones first
  /^tell\s+me\s+about\b/i,
  /^describe\b/i,
  /^define\b/i,
];

const FIX_CJK = [/修復/, /修正/, /修一下/, /為什麼.*(?:錯誤|失敗|壞|報錯)/];
const SUMMARIZE_CJK = [/^總結/, /^摘要/, /^簡述/, /^概述/, /^一句話[總概]/];
const EXPLAIN_CJK = [/^解釋/, /^說明/, /^什麼是/, /^如何.*(?:運作|工作|實作|實現)/, /^介紹一下/];

function matchesAny(text: string, patterns: RegExp[]): boolean {
  for (const re of patterns) {
    if (re.test(text)) return true;
  }
  return false;
}

export function classifyNlIntent(query: string): NlIntentMatch | null {
  const trimmed = query.trim();
  if (!trimmed) return null;
  // Skip sigils handled upstream.
  if (trimmed.startsWith("/") || trimmed.startsWith(">")) return null;

  if (matchesAny(trimmed, FIX_SHAPED)) return { id: "fix", text: trimmed };
  if (matchesAny(trimmed, FIX_VERB_EN) || matchesAny(trimmed, FIX_CJK)) {
    return { id: "fix", text: trimmed };
  }
  if (matchesAny(trimmed, SUMMARIZE_VERB_EN) || matchesAny(trimmed, SUMMARIZE_CJK)) {
    return { id: "summarize", text: trimmed };
  }
  if (matchesAny(trimmed, EXPLAIN_VERB_EN) || matchesAny(trimmed, EXPLAIN_CJK)) {
    return { id: "explain", text: trimmed };
  }
  if (looksLikeAiCommandIntent(trimmed)) return { id: "cmd", text: trimmed };
  return null;
}
