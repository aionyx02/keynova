import { IPCProvider } from "../context/IPCContext";
import { FeatureProvider } from "../context/FeatureContext";
import { CommandPalette } from "./CommandPalette";
import { ErrorBoundary } from "../shared/components/ErrorBoundary";

export function AppContainer() {
  const palette = (
    <ErrorBoundary>
      <IPCProvider>
        <FeatureProvider>
          <CommandPalette />
        </FeatureProvider>
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
