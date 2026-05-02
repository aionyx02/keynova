import { useMouseControl } from "../hooks/useMouseControl";

/** 顯示全域滑鼠控制模式狀態（Alt+M 切換，Alt+W/A/S/D 移動）。目前未掛載至 UI。 */
export function MouseControlOverlay() {
  const { active } = useMouseControl();

  return (
    <div
      title={active ? "全域滑鼠控制：開（Alt+M 關閉）" : "全域滑鼠控制：關（Alt+M 開啟）"}
      className={`fixed bottom-4 right-4 px-3 py-1.5 rounded text-xs font-medium shadow-lg ${
        active ? "bg-blue-600 text-white" : "bg-gray-700/60 text-gray-400"
      }`}
    >
      {active ? "滑鼠模式：ON" : "滑鼠模式：OFF"}
    </div>
  );
}
