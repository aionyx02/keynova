import { createContext, useCallback, useContext } from "react";
import type { ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";

export interface IpcErrorPayload {
  code: string;
  message: string;
  details?: unknown;
}

export interface KeynovaIpcError extends Error {
  code?: string;
  details?: unknown;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function parseStringError(value: string): IpcErrorPayload | null {
  try {
    const parsed: unknown = JSON.parse(value);
    if (
      isRecord(parsed) &&
      typeof parsed.code === "string" &&
      typeof parsed.message === "string"
    ) {
      return { code: parsed.code, message: parsed.message, details: parsed.details };
    }
  } catch {
    // Plain string errors are kept readable.
  }
  return null;
}

function normalizeIpcError(error: unknown): KeynovaIpcError {
  const payload =
    typeof error === "string"
      ? parseStringError(error)
      : isRecord(error) &&
          typeof error.code === "string" &&
          typeof error.message === "string"
        ? { code: error.code, message: error.message, details: error.details }
        : null;

  if (payload) {
    const normalized = new Error(payload.message) as KeynovaIpcError;
    normalized.code = payload.code;
    normalized.details = payload.details;
    return normalized;
  }

  return error instanceof Error ? error : (new Error(String(error)) as KeynovaIpcError);
}

export type DispatchFn = <T>(route: string, payload?: Record<string, unknown>) => Promise<T>;

interface IPCContextValue {
  dispatch: DispatchFn;
}

const IPCContext = createContext<IPCContextValue | null>(null);

export function IPCProvider({ children }: { children: ReactNode }) {
  // Stable reference — useIPC callers can include dispatch in effect deps without re-running.
  const dispatch = useCallback(async function <T>(
    route: string,
    payload?: Record<string, unknown>,
  ): Promise<T> {
    if (!window.__TAURI_INTERNALS__) {
      throw new Error(
        "[useIPC] Tauri runtime not found. Run the app with `npm run tauri dev`, not in a browser.",
      );
    }
    try {
      return await invoke<T>("cmd_dispatch", { route, payload: payload ?? null });
    } catch (error) {
      throw normalizeIpcError(error);
    }
  }, []) as DispatchFn;

  return <IPCContext.Provider value={{ dispatch }}>{children}</IPCContext.Provider>;
}

export function useIPCContext(): IPCContextValue {
  const ctx = useContext(IPCContext);
  if (!ctx) throw new Error("[useIPC] must be used inside <IPCProvider>");
  return ctx;
}