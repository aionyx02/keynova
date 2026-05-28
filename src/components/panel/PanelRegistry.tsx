import React from "react";
import { SettingPanel } from "../SettingPanel";
import type { PanelProps } from "../../types/panel";

export type { PanelProps };

// REF.6.B follow-up — `AiPanel` lazy import removed from the registry. The
// component file is retained (deletion is REF.8 territory pending the
// observation cycle) but no palette command routes to it anymore.
const ModelDownloadPanel = React.lazy(() =>
  import("../ModelDownloadPanel").then((m) => ({ default: m.ModelDownloadPanel })),
);
const ModelListPanel = React.lazy(() =>
  import("../ModelListPanel").then((m) => ({ default: m.ModelListPanel })),
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
const ModelRemovePanel = React.lazy(() =>
  import("../ModelRemovePanel").then((m) => ({ default: m.ModelRemovePanel })),
);
const SystemMonitoringPanel = React.lazy(() =>
  import("../SystemMonitoringPanel").then((m) => ({ default: m.SystemMonitoringPanel })),
);
const NvimDownloadPanel = React.lazy(() =>
  import("../NvimDownloadPanel").then((m) => ({ default: m.NvimDownloadPanel })),
);

/** 將後端回傳的 panel name 對應至 React 元件。新增面板只需在此 Record 加一筆。 */
export const PanelRegistry: Record<string, React.ComponentType<PanelProps>> = {
  setting: SettingPanel as React.ComponentType<PanelProps>,
  model_download: ModelDownloadPanel,
  model_list: ModelListPanel,
  model_remove: ModelRemovePanel,
  translation: TranslationPanel,
  note: NoteEditor,
  calculator: CalculatorPanel,
  history: HistoryPanel,
  system: SystemPanel,
  system_monitoring: SystemMonitoringPanel,
  nvim_download: NvimDownloadPanel,
};
