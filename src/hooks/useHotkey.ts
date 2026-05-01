import { useIPC } from "./useIPC";
import type { ConflictInfo, HotkeyConfig } from "../types/hotkey";

export function useHotkey() {
  const { dispatch } = useIPC();

  async function register(config: HotkeyConfig): Promise<void> {
    return dispatch("hotkey.register", config as unknown as Record<string, unknown>);
  }

  async function unregister(id: string): Promise<void> {
    return dispatch("hotkey.unregister", { id });
  }

  async function list(): Promise<HotkeyConfig[]> {
    return dispatch<HotkeyConfig[]>("hotkey.list");
  }

  async function checkConflict(config: HotkeyConfig): Promise<ConflictInfo | null> {
    return dispatch<ConflictInfo | null>("hotkey.check_conflict", config as unknown as Record<string, unknown>);
  }

  return { register, unregister, list, checkConflict };
}
