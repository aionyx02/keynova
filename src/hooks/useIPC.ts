import { invoke } from "@tauri-apps/api/core";

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
    return invoke<T>("cmd_dispatch", { route, payload: payload ?? null });
  }

  return { dispatch };
}
