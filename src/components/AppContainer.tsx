import { IPCProvider } from "../context/IPCContext";
import { FeatureProvider } from "../context/FeatureContext";
import { FeatureFlagsProvider } from "../context/FeatureFlagsContext";
import { CommandPalette } from "./CommandPalette";
import { ErrorBoundary } from "../shared/components/ErrorBoundary";

export function AppContainer() {
  const palette = (
    <ErrorBoundary>
      <IPCProvider>
        <FeatureFlagsProvider>
          <FeatureProvider>
            <CommandPalette />
          </FeatureProvider>
        </FeatureFlagsProvider>
      </IPCProvider>
    </ErrorBoundary>
  );

  if (typeof window !== "undefined" && !window.__TAURI_INTERNALS__) {
    return (
      <div className="kn-browser-preview">
        <div className="kn-browser-preview-frame">{palette}</div>
      </div>
    );
  }

  return palette;
}
