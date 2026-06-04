import React from "react";

import type { FeatureManifest } from "../featureManifest";

// DECOUP.5 (ADR-0044): system monitor owns its panel; gated by the same
// `system` flag as the system panel.
const SystemMonitoringPanel = React.lazy(() =>
  import("./SystemMonitoringPanel").then((m) => ({ default: m.SystemMonitoringPanel })),
);

export const systemMonitorManifest: FeatureManifest = {
  panels: { system_monitoring: SystemMonitoringPanel },
  panelGate: { system_monitoring: "system" },
};
