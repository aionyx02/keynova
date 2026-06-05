import React from "react";

import type { FeatureManifest } from "../featureManifest";

// DECOUP.5 (ADR-0044): system owns its panel + gate.
const SystemPanel = React.lazy(() =>
  import("./SystemPanel").then((m) => ({ default: m.SystemPanel })),
);

export const systemManifest: FeatureManifest = {
  panels: { system: SystemPanel },
  panelGate: { system: "system" },
};
