import { useWorkspace } from "../hooks/useWorkspace";

/** 顯示當前工作區編號（1/2/3）的小標籤，點擊可切換。 */
export function WorkspaceIndicator() {
  const { current, switchTo } = useWorkspace();
  if (!window.__TAURI_INTERNALS__) return null;

  return (
    <div className="flex items-center gap-0.5 shrink-0">
      {[0, 1, 2].map((slot) => (
        <button
          key={slot}
          onClick={() => void switchTo(slot)}
          title={`Ctrl+Alt+${slot + 1}`}
          className={`w-5 h-5 rounded text-[9px] font-bold transition-colors ${
            current?.id === slot
              ? "bg-blue-600/70 text-white"
              : "text-gray-600 hover:text-gray-400 hover:bg-white/5"
          }`}
        >
          {slot + 1}
        </button>
      ))}
    </div>
  );
}