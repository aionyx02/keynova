import React from "react";
import type { PanelProps } from "../../types/panel";

export type { PanelProps };

// REF.8 — `AiPanel` (legacy chat-first UI) deleted along with the `ai_legacy`
// panel route and the `ai_legacy_chat` builtin command. The backend agent
// (agent_runtime + handlers/agent) is retained as a dormant capability asset
// behind the reserved `ai.legacy_agent` flag, but has no UI entry point.
const SettingPanel = React.lazy(() =>
  import("../../features/settings/SettingPanel").then((m) => ({ default: m.SettingPanel })),
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
};
