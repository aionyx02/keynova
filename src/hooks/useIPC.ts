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
      return {
        code: parsed.code,
        message: parsed.message,
        details: parsed.details,
      };
    }
  } catch {
    // Old backend errors are plain strings; keep those readable.
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
        ? {
            code: error.code,
            message: error.message,
            details: error.details,
          }
        : null;

  if (payload) {
    const normalized = new Error(payload.message) as KeynovaIpcError;
    normalized.code = payload.code;
    normalized.details = payload.details;
    return normalized;
  }

  if (error instanceof Error) {
    return error;
  }

  return new Error(String(error)) as KeynovaIpcError;
}

/**
 * 通用 Tauri IPC 呼叫包裝。
 * 使用泛型確保回傳型別安全，不使用 any。
 *
 * 注意：必須透過 `npm run tauri dev` 啟動，直接在瀏覽器開啟會因缺少
 * Tauri runtime 而拋出 "Cannot read properties of undefined (reading 'invoke')"。
 */
export function useIPC() {
  async function dispatch<T>(route: string, payload?: Record<string, unknown>): Promise<T> {
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
  }

  return { dispatch };
}
