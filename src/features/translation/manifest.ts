import React from "react";

import type { FeatureManifest } from "../featureManifest";

// DECOUP.5 (ADR-0044): translation owns its panel + gate.
const TranslationPanel = React.lazy(() =>
  import("./TranslationPanel").then((m) => ({ default: m.TranslationPanel })),
);

export const translationManifest: FeatureManifest = {
  panels: { translation: TranslationPanel },
  panelGate: { translation: "translation" },
};
