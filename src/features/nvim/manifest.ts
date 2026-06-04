import React from "react";

import type { FeatureManifest } from "../featureManifest";

// DECOUP.5 (ADR-0044): nvim owns its panel. No panel gate — nvim is a session
// feature, not a `features.*` config flag.
const NvimDownloadPanel = React.lazy(() =>
  import("./NvimDownloadPanel").then((m) => ({ default: m.NvimDownloadPanel })),
);

export const nvimManifest: FeatureManifest = {
  panels: { nvim_download: NvimDownloadPanel },
};
