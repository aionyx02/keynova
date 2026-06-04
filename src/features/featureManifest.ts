// DECOUP / ADR-0044 — frontend feature self-registration.
//
// Each feature exports a `FeatureManifest` declaring what it contributes to the
// shared palette surfaces (panel + panel gate today; result badges / routes join
// in later batches). The central FEATURE_MANIFESTS array is the single place
// features are enumerated, so removing one = delete its folder + one entry here.
//
// Un-migrated features still register in the central tables (PanelRegistry /
// usePalettePanels PANEL_FEATURE); the two styles coexist during the rollout.

import type React from "react";

import type { GateKey } from "../context/FeatureFlagsContext";
import type { PanelProps } from "../types/panel";
import { calculatorManifest } from "./calculator/manifest";
import { historyManifest } from "./history/manifest";
import { notesManifest } from "./notes/manifest";
import { nvimManifest } from "./nvim/manifest";
import { systemManifest } from "./system/manifest";
import { systemMonitorManifest } from "./system-monitor/manifest";
import { translationManifest } from "./translation/manifest";

export interface FeatureManifest {
  /** Panels this feature contributes: panel name → lazy component. */
  panels?: Record<string, React.ComponentType<PanelProps>>;
  /** Maps a contributed panel name to the feature flag that hides it. */
  panelGate?: Record<string, GateKey>;
}

/** Self-registering frontend features (ADR-0044). */
export const FEATURE_MANIFESTS: readonly FeatureManifest[] = [
  calculatorManifest,
  translationManifest,
  notesManifest,
  historyManifest,
  systemManifest,
  systemMonitorManifest,
  nvimManifest,
];

/** Merged panel name → component map contributed by all manifests. */
export function manifestPanels(): Record<string, React.ComponentType<PanelProps>> {
  return Object.assign({}, ...FEATURE_MANIFESTS.map((m) => m.panels ?? {}));
}

/** Merged panel name → gate-key map contributed by all manifests. */
export function manifestPanelGates(): Record<string, GateKey> {
  return Object.assign({}, ...FEATURE_MANIFESTS.map((m) => m.panelGate ?? {}));
}
