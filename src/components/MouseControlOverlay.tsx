import { useMouseControl } from "../hooks/useMouseControl";

export function MouseControlOverlay() {
  const { active, toggle } = useMouseControl();

  return (
    <button
      onClick={toggle}
      title={active ? "停用鍵盤滑鼠控制（Ctrl+Alt+M）" : "啟用鍵盤滑鼠控制（Ctrl+Alt+M）"}
      className={`fixed bottom-4 right-4 px-3 py-1.5 rounded text-xs font-medium shadow-lg transition-colors ${
        active
          ? "bg-blue-600 text-white"
          : "bg-gray-700 text-gray-300 hover:bg-gray-600"
      }`}
    >
      {active ? "滑鼠控制：開" : "滑鼠控制：關"}
    </button>
  );
}
