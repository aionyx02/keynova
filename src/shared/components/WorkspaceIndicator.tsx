import { UiIcon } from "../../components/icons/UiIcon";
import { useWorkspace } from "../../hooks/useWorkspace";

export function WorkspaceIndicator() {
  const { current, switchTo } = useWorkspace();

  if (!window.__TAURI_INTERNALS__) return null;

  return (
    <div className="flex items-center gap-1 rounded-[14px] border border-[color:var(--kn-border)] bg-white/[0.035] p-1 shadow-[inset_0_1px_0_rgba(255,255,255,0.03)]">
      <span className="flex h-7 w-7 items-center justify-center rounded-[10px] text-[color:var(--kn-text-faint)]">
        <UiIcon name="workspace" className="h-3.5 w-3.5" />
      </span>
      {[0, 1, 2].map((slot) => {
        const isActive = current?.id === slot;
        return (
          <button
            key={slot}
            type="button"
            onClick={() => void switchTo(slot)}
            title={`Workspace ${slot + 1} (Ctrl+Alt+${slot + 1})`}
            className={`flex h-7 w-7 items-center justify-center rounded-[10px] text-[10px] font-semibold transition-all duration-150 ${
              isActive
                ? "border border-[color:rgba(127,212,255,0.24)] bg-[linear-gradient(180deg,_rgba(127,212,255,0.22)_0%,_rgba(85,188,255,0.12)_100%)] text-[color:var(--kn-text)] shadow-[inset_0_1px_0_rgba(255,255,255,0.16)]"
                : "text-[color:var(--kn-text-muted)] hover:bg-white/[0.05] hover:text-[color:var(--kn-text-soft)]"
            }`}
            aria-pressed={isActive}
          >
            {slot + 1}
          </button>
        );
      })}
    </div>
  );
}
