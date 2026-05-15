import { IPCProvider } from "../context/IPCContext";
import { FeatureProvider } from "../context/FeatureContext";
import { CommandPalette } from "./CommandPalette";

export function AppContainer() {
  return (
    <IPCProvider>
      <FeatureProvider>
        <CommandPalette />
      </FeatureProvider>
    </IPCProvider>
  );
}