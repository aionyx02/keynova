import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";

/**
 * 追蹤全域滑鼠控制模式狀態。
 * 實際的按鍵偵測與游標移動已移至 Rust 層（Alt+M 切換 / Alt+W/A/S/D 移動）。
 * 此 hook 僅監聽 Rust 發出的 mouse-control-toggled 事件以同步顯示狀態。
 */
export function useMouseControl() {
  const [active, setActive] = useState(false);

  useEffect(() => {
    const unlisten = listen<boolean>("mouse-control-toggled", (event) => {
      setActive(event.payload);
    });
    return () => { unlisten.then((fn) => fn()); };
  }, []);

  return { active };
}
