import React from "react";

import type { FeatureManifest } from "../featureManifest";

// DECOUP.5 (ADR-0044): history owns its panel + gate.
const HistoryPanel = React.lazy(() =>
  import("./HistoryPanel").then((m) => ({ default: m.HistoryPanel })),
);

export const historyManifest: FeatureManifest = {
  panels: { history: HistoryPanel },
  panelGate: { history: "history" },
};
