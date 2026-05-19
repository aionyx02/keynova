/**
 * AI error classifier — maps raw error strings from `ai.response` / agent
 * failures into a structured kind with a human-friendly title, hint, and
 * optional call-to-action. Pure function; no React/Tauri dependency so it can
 * be tested in isolation and reused outside the panel.
 */

export type AiErrorKind =
  | "connection_refused"
  | "model_not_found"
  | "unauthorized"
  | "timeout"
  | "out_of_memory"
  | "cancelled"
  | "unknown";

export interface AiErrorCta {
  label: string;
  /**
   * Suggested target. `setting` opens /setting; `external:<url>` opens an
   * external link; `retry` re-sends the last user prompt.
   */
  action: "setting" | "retry" | `external:${string}`;
}

export interface ClassifiedAiError {
  kind: AiErrorKind;
  title: string;
  hint: string;
  cta?: AiErrorCta;
}

export function classifyAiError(raw: string | null | undefined): ClassifiedAiError {
  const msg = (raw ?? "").toLowerCase();

  if (
    msg.includes("connection refused") ||
    msg.includes("is not running") ||
    msg.includes("is not reachable") ||
    msg.includes("econnrefused")
  ) {
    return {
      kind: "connection_refused",
      title: "AI provider unreachable",
      hint: "The provider daemon refused the connection. Start Ollama (or your local model server) and retry.",
      cta: { label: "Open AI settings", action: "setting" },
    };
  }

  if (
    msg.includes("model not found") ||
    msg.includes("model is not found") ||
    msg.includes("no such model") ||
    msg.includes("model not loaded") ||
    msg.includes("ollama pull")
  ) {
    return {
      kind: "model_not_found",
      title: "Model not installed",
      hint: "The selected model isn't available on this machine. Pull the model from your provider and retry.",
      cta: { label: "Open Ollama downloads", action: "external:https://ollama.com/library" },
    };
  }

  if (
    msg.includes("401") ||
    msg.includes("403") ||
    msg.includes("unauthorized") ||
    msg.includes("invalid api key") ||
    msg.includes("api key not configured") ||
    msg.includes("api key is not set")
  ) {
    return {
      kind: "unauthorized",
      title: "Authentication failed",
      hint: "The provider rejected the API key. Check `/setting` and re-enter the key.",
      cta: { label: "Open AI settings", action: "setting" },
    };
  }

  if (
    msg.includes("timed out") ||
    msg.includes("timeout") ||
    msg.includes("deadline exceeded")
  ) {
    return {
      kind: "timeout",
      title: "Request timed out",
      hint: "The model didn't respond in time. Increase `ai.timeout_secs` (or `ai.ollama_timeout_secs` for local models) and retry.",
      cta: { label: "Retry", action: "retry" },
    };
  }

  if (
    msg.includes("out of memory") ||
    msg.includes("oom") ||
    msg.includes("cuda out of memory") ||
    msg.includes("model requires more")
  ) {
    return {
      kind: "out_of_memory",
      title: "Out of memory",
      hint: "The model needs more RAM/VRAM than is available. Pick a smaller model variant in `/setting`.",
      cta: { label: "Open AI settings", action: "setting" },
    };
  }

  if (msg === "cancelled" || msg.includes("cancel")) {
    return {
      kind: "cancelled",
      title: "Cancelled",
      hint: "You cancelled this request.",
    };
  }

  return {
    kind: "unknown",
    title: "AI error",
    hint: raw ?? "Unknown error",
  };
}
