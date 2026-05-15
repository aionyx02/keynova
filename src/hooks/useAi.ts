import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";

export interface AiSetupStatus {
  needs_setup: boolean;
  provider: string;
  target_model: string;
  ollama_reachable: boolean;
  model_available: boolean;
  recommended_model: string;
  reason: string;
}

export interface AiMessage {
  role: "user" | "assistant";
  content: string;
  pending?: boolean;
  error?: string;
}

interface AiResponsePayload {
  request_id: string;
  ok: boolean;
  reply?: string;
  error?: string;
}

async function ipcDispatch<T>(route: string, payload?: Record<string, unknown>): Promise<T> {
  return invoke<T>("cmd_dispatch", { route, payload: payload ?? null });
}

export function useAi() {
  const [messages, setMessages] = useState<AiMessage[]>([]);
  const [loading, setLoading] = useState(false);
  const pendingIdRef = useRef<string | null>(null);
  const hasLocalMutationRef = useRef(false);

  useEffect(() => {
    if (!window.__TAURI_INTERNALS__) return;
    let active = true;
    // Load existing history on mount
    ipcDispatch<{ role: string; content: string }[]>("ai.get_history")
      .then((hist) => {
        if (!active || hasLocalMutationRef.current) return;
        setMessages(hist.map((m) => ({ role: m.role as "user" | "assistant", content: m.content })));
      })
      .catch(() => {});

    const unlisten = listen<AiResponsePayload>("ai-response", (event) => {
      const { request_id, ok, reply, error } = event.payload;
      if (request_id !== pendingIdRef.current) return;
      pendingIdRef.current = null;
      setLoading(false);
      setMessages((prev) => {
        const next = prev.filter((m) => !m.pending);
        if (ok && reply) {
          return [...next, { role: "assistant", content: reply }];
        }
        return [...next, { role: "assistant", content: "", error: error ?? "Unknown error" }];
      });
    });

    return () => {
      active = false;
      unlisten.then((fn) => fn());
    };
  }, []);

  const send = useCallback(async (prompt: string) => {
    if (loading || !prompt.trim()) return;

    hasLocalMutationRef.current = true;
    const requestId = `ai-${Date.now()}`;
    pendingIdRef.current = requestId;
    setLoading(true);
    setMessages((prev) => [
      ...prev,
      { role: "user", content: prompt },
      { role: "assistant", content: "", pending: true },
    ]);

    try {
      await ipcDispatch("ai.chat", { request_id: requestId, prompt });
    } catch (err) {
      pendingIdRef.current = null;
      setLoading(false);
      setMessages((prev) => {
        const next = prev.filter((m) => !m.pending);
        return [...next, { role: "assistant", content: "", error: String(err) }];
      });
    }
  }, [loading]);

  const clearHistory = useCallback(async () => {
    hasLocalMutationRef.current = true;
    await ipcDispatch("ai.clear_history");
    pendingIdRef.current = null;
    setLoading(false);
    setMessages([]);
  }, []);

  const checkSetup = useCallback(async (): Promise<AiSetupStatus | null> => {
    if (!window.__TAURI_INTERNALS__) return null;
    try {
      return await ipcDispatch<AiSetupStatus>("ai.check_setup");
    } catch {
      return null;
    }
  }, []);

  return { messages, loading, send, clearHistory, checkSetup };
}
