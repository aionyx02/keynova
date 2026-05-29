import { IPCProvider } from "../context/IPCContext";
import { FeatureProvider } from "../context/FeatureContext";
import { CommandPalette } from "./CommandPalette";
import { ErrorBoundary } from "../shared/components/ErrorBoundary";

export function AppContainer() {
  return (
    <ErrorBoundary>
      <IPCProvider>
        <FeatureProvider>
          <CommandPalette />
        </FeatureProvider>
      </IPCProvider>
    </ErrorBoundary>
  );
}