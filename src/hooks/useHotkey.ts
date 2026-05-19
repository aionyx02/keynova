import { useIPC } from "./useIPC";
import { IPC } from "../ipc/routes";
import type { ConflictInfo, HotkeyConfig } from "../types/hotkey";

export function useHotkey() {
  const { dispatch } = useIPC();

  function register(config: HotkeyConfig): Promise<void> {
    return dispatch(IPC.HOTKEY_REGISTER, config as unknown as Record<string, unknown>);
  }

  function unregister(id: string): Promise<void> {
    return dispatch(IPC.HOTKEY_UNREGISTER, { id });
  }

  function list(): Promise<HotkeyConfig[]> {
    return dispatch<HotkeyConfig[]>(IPC.HOTKEY_LIST);
  }

  function checkConflict(config: HotkeyConfig): Promise<ConflictInfo | null> {
    return dispatch<ConflictInfo | null>(
      IPC.HOTKEY_CHECK_CONFLICT,
      config as unknown as Record<string, unknown>,
    );
  }

  return { register, unregister, list, checkConflict };
}