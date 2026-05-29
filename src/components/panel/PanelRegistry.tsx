import React from "react";
import { SettingPanel } from "../../features/settings/SettingPanel";
import type { PanelProps } from "../../types/panel";

export type { PanelProps };

// REF.6.B follow-up — `AiPanel` lazy import removed from the registry hot
// path. The component file is retained (deletion is REF.8 territory pending
// the observation cycle).
// REF.7.A — `ai_legacy` lazy entry restored as the legacy compatibility
// route. The backend builtin command `ai_legacy_chat` only registers when
// `ai.legacy_agent = true`, so the user has to opt into the chat surface
// before this lazy import is ever resolved. With the flag off (default), the
// chunk stays cold and adds zero cost to first paint.
const AiPanelLazy = React.lazy(() =>
  import("../AiPanel").then((m) => ({ default: m.AiPanel })),
);
const ModelDownloadPanel = React.lazy(() =>
  import("../../features/model-manager/ModelDownloadPanel").then((m) => ({ default: m.ModelDownloadPanel })),
);
const ModelListPanel = React.lazy(() =>
  import("../../features/model-manager/ModelListPanel").then((m) => ({ default: m.ModelListPanel })),
);
const TranslationPanel = React.lazy(() =>
  import("../../features/translation/TranslationPanel").then((m) => ({ default: m.TranslationPanel })),
);
const NoteEditor = React.lazy(() =>
  import("../../features/notes/NoteEditor").then((m) => ({ default: m.NoteEditor })),
);
const CalculatorPanel = React.lazy(() =>
  import("../../features/calculator/CalculatorPanel").then((m) => ({ default: m.CalculatorPanel })),
);
const HistoryPanel = React.lazy(() =>
  import("../../features/history/HistoryPanel").then((m) => ({ default: m.HistoryPanel })),
);
const SystemPanel = React.lazy(() =>
  import("../../features/system/SystemPanel").then((m) => ({ default: m.SystemPanel })),
);
const ModelRemovePanel = React.lazy(() =>
  import("../../features/model-manager/ModelRemovePanel").then((m) => ({ default: m.ModelRemovePanel })),
);
const SystemMonitoringPanel = React.lazy(() =>
  import("../../features/system-monitor/SystemMonitoringPanel").then((m) => ({ default: m.SystemMonitoringPanel })),
);
const NvimDownloadPanel = React.lazy(() =>
  import("../../features/nvim/NvimDownloadPanel").then((m) => ({ default: m.NvimDownloadPanel })),
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
  ai_legacy: AiPanelLazy,
};
