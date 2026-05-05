import React from "react";
import { SettingPanel } from "../SettingPanel";
import type { PanelProps } from "../../types/panel";

export type { PanelProps };

const AiPanel = React.lazy(() =>
  import("../AiPanel").then((m) => ({ default: m.AiPanel })),
);
const TranslationPanel = React.lazy(() =>
  import("../TranslationPanel").then((m) => ({ default: m.TranslationPanel })),
);
const NoteEditor = React.lazy(() =>
  import("../NoteEditor").then((m) => ({ default: m.NoteEditor })),
);
const CalculatorPanel = React.lazy(() =>
  import("../CalculatorPanel").then((m) => ({ default: m.CalculatorPanel })),
);
const HistoryPanel = React.lazy(() =>
  import("../HistoryPanel").then((m) => ({ default: m.HistoryPanel })),
);
const SystemPanel = React.lazy(() =>
  import("../SystemPanel").then((m) => ({ default: m.SystemPanel })),
);

/** 將後端回傳的 panel name 對應至 React 元件。新增面板只需在此 Record 加一筆。 */
export const PanelRegistry: Record<string, React.ComponentType<PanelProps>> = {
  setting: SettingPanel as React.ComponentType<PanelProps>,
  ai: AiPanel,
  translation: TranslationPanel,
  note: NoteEditor,
  calculator: CalculatorPanel,
  history: HistoryPanel,
  system: SystemPanel,
};