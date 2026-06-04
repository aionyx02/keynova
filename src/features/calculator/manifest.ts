import React from "react";

import type { FeatureManifest } from "../featureManifest";

// DECOUP.2 (ADR-0044): calculator owns its own panel registration + gate, so the
// central PanelRegistry / panel-gate tables no longer name it. Removing the
// feature = delete this folder + its entry in featureManifest.ts.
const CalculatorPanel = React.lazy(() =>
  import("./CalculatorPanel").then((m) => ({ default: m.CalculatorPanel })),
);

export const calculatorManifest: FeatureManifest = {
  panels: { calculator: CalculatorPanel },
  panelGate: { calculator: "calculator" },
};
