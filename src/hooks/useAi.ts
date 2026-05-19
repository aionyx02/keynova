import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { IPC } from "../ipc/routes";

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
  /** Epoch ms when the pending request started; used by the elapsed timer. */
  startedAt?: number;
}

interface AiResponsePayload {
  request_id: string;
  ok: boolean;
  reply?: string;
  error?: string;
  cancelled?: boolean;
}

interface AiStreamChunkPayload {
  request_id: string;
  delta: string;
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
    ipcDispatch<{ role: string; content: string }[]>(IPC.AI_GET_HISTORY)
      .then((hist) => {
        if (!active || hasLocalMutationRef.current) return;
        setMessages(hist.map((m) => ({ role: m.role as "user" | "assistant", content: m.content })));
      })
      .catch(() => {});

    const unlistenStream = listen<AiStreamChunkPayload>("ai-stream-chunk", (event) => {
      const { request_id, delta } = event.payload;
      if (request_id !== pendingIdRef.current) return;
      setMessages((prev) => {
        // Find the latest pending assistant bubble and append delta. Keep it
        // pending until ai-response arrives so the spinner/elapsed timer stay
        // visible alongside streamed text.
        const idx = prev.findIndex((m) => m.pending);
        if (idx === -1) return prev;
        const next = prev.slice();
        next[idx] = { ...next[idx], content: next[idx].content + delta };
        return next;
      });
    });

    const unlisten = listen<AiResponsePayload>("ai-response", (event) => {
      const { request_id, ok, reply, error, cancelled } = event.payload;
      if (request_id !== pendingIdRef.current) return;
      pendingIdRef.current = null;
      setLoading(false);
      setMessages((prev) => {
        const next = prev.filter((m) => !m.pending);
        if (cancelled) {
          // Cancelled: drop the in-flight bubble entirely (user message stays
          // in UI history; backend has already rolled its own copy back).
          return next;
        }
        if (ok && reply) {
          return [...next, { role: "assistant", content: reply }];
        }
        return [...next, { role: "assistant", content: "", error: error ?? "Unknown error" }];
      });
    });

    return () => {
      active = false;
      unlisten.then((fn) => fn());
      unlistenStream.then((fn) => fn());
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
      { role: "assistant", content: "", pending: true, startedAt: Date.now() },
    ]);

    try {
      await ipcDispatch(IPC.AI_CHAT, { request_id: requestId, prompt });
    } catch (err) {
      pendingIdRef.current = null;
      setLoading(false);
      setMessages((prev) => {
        const next = prev.filter((m) => !m.pending);
        return [...next, { role: "assistant", content: "", error: String(err) }];
      });
    }
  }, [loading]);

  const cancel = useCallback(async () => {
    const requestId = pendingIdRef.current;
    if (!requestId) return;
    try {
      await ipcDispatch(IPC.AI_CANCEL, { request_id: requestId });
    } catch {
      // Ignore — the ai-response event with cancelled:true will clean up state.
    }
  }, []);

  const clearHistory = useCallback(async () => {
    hasLocalMutationRef.current = true;
    await ipcDispatch(IPC.AI_CLEAR_HISTORY);
    pendingIdRef.current = null;
    setLoading(false);
    setMessages([]);
  }, []);

  const checkSetup = useCallback(async (): Promise<AiSetupStatus | null> => {
    if (!window.__TAURI_INTERNALS__) return null;
    try {
      return await ipcDispatch<AiSetupStatus>(IPC.AI_CHECK_SETUP);
    } catch {
      return null;
    }
  }, []);

  return { messages, loading, send, cancel, clearHistory, checkSetup };
}
