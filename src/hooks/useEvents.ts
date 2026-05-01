import { listen } from "@tauri-apps/api/event";
import { useEffect } from "react";

/**
 * 訂閱 Tauri 後端事件，並在組件卸載時自動清理監聽器。
 */
export function useEvents<T>(
  event: string,
  handler: (payload: T) => void,
) {
  useEffect(() => {
    let unlisten: (() => void) | undefined;

    listen<T>(event, (e) => handler(e.payload)).then((fn) => {
      unlisten = fn;
    });

    return () => {
      unlisten?.();
    };
  }, [event, handler]);
}
