import React from "react";
import type { PanelProps } from "../../types/panel";
import { manifestPanels } from "../../features/featureManifest";

export type { PanelProps };

// REF.8 — `AiPanel` (legacy chat-first UI) deleted along with the `ai_legacy`
// panel route and the `ai_legacy_chat` builtin command. The backend agent
// (agent_runtime + handlers/agent) is retained as a dormant capability asset
// behind the reserved `ai.legacy_agent` flag, but has no UI entry point.
//
// DECOUP.5 (ADR-0044): feature panels now self-register via their
// `features/<x>/manifest.ts`; only the non-feature `setting` and the
// AI-bootstrap-exempt `model` panel remain hand-listed here.
const SettingPanel = React.lazy(() =>
  import("../../features/settings/SettingPanel").then((m) => ({ default: m.SettingPanel })),
);
const ModelPanel = React.lazy(() =>
  import("../../features/model-manager/ModelPanel").then((m) => ({ default: m.ModelPanel })),
);

/** 將後端回傳的 panel name 對應至 React 元件。`setting`/`model` 為非功能/豁免面板；
 * 其餘功能面板由各自的 manifest 經 `manifestPanels()` 併入（DECOUP/ADR-0044）。 */
export const PanelRegistry: Record<string, React.ComponentType<PanelProps>> = {
  setting: SettingPanel as React.ComponentType<PanelProps>,
  model: ModelPanel,
  ...manifestPanels(),
};
