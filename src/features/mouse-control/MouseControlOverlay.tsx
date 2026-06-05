import { useI18n } from "../../i18n/useI18n";
import { useMouseControl } from "../../hooks/useMouseControl";

/** Displays global mouse-control mode state. Currently not mounted into the UI. */
export function MouseControlOverlay() {
  const t = useI18n().mouseControl;
  const { active } = useMouseControl();

  return (
    <div
      title={active ? t.titleOn : t.titleOff}
      className={`fixed bottom-4 right-4 px-3 py-1.5 rounded text-xs font-medium shadow-lg ${
        active ? "bg-blue-600 text-white" : "bg-gray-700/60 text-gray-400"
      }`}
    >
      {active ? t.labelOn : t.labelOff}
    </div>
  );
}
