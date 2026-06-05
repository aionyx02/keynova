import React from "react";

import type { FeatureManifest } from "../featureManifest";

// DECOUP.5 (ADR-0044): notes owns its panel + gate (panel name "note").
const NoteEditor = React.lazy(() =>
  import("./NoteEditor").then((m) => ({ default: m.NoteEditor })),
);

export const notesManifest: FeatureManifest = {
  panels: { note: NoteEditor },
  panelGate: { note: "notes" },
};
