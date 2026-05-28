// REF.2.P5 — Ref bundle for the ESC priority chain + window resize.
//
// `useEscapeKey` registers its global keydown listener exactly once (see its
// module comment for why) and reads the latest mode / cmdResult / query /
// secondaryMenuOpen / expandedMetadata via refs to avoid stale closures.
// `useWindowResize` reads `modeRef` and `cmdResultRef` on every tick.
//
// This hook keeps the 5 refs and the one-line `useLayoutEffect` that syncs
// them on every commit. `useLayoutEffect` (not `useEffect`) matters because
// keydown handlers can fire during paint and would otherwise read a
// previous-frame value.

import { useLayoutEffect, useRef } from "react";

import type { BuiltinCommandResult } from "../../../hooks/useCommands";

interface Deps {
  mode: "search" | "command" | "terminal";
  cmdResult: BuiltinCommandResult | null;
  query: string;
  secondaryMenuOpen: boolean;
  expandedMetadata: boolean;
  /** REF.6.B — true when `usePaletteMode` is in capability kind. */
  capabilityMode: boolean;
  /** REF.6.B — true when the capability stream is pending or streaming. */
  capabilityStreaming: boolean;
}

interface UsePaletteRefs {
  modeRef: React.RefObject<Deps["mode"]>;
  cmdResultRef: React.RefObject<BuiltinCommandResult | null>;
  queryRef: React.RefObject<string>;
  secondaryMenuOpenRef: React.RefObject<boolean>;
  expandedMetadataRef: React.RefObject<boolean>;
  capabilityModeRef: React.RefObject<boolean>;
  capabilityStreamingRef: React.RefObject<boolean>;
}

export function usePaletteRefs(deps: Deps): UsePaletteRefs {
  const modeRef = useRef(deps.mode);
  const cmdResultRef = useRef(deps.cmdResult);
  const queryRef = useRef(deps.query);
  const secondaryMenuOpenRef = useRef(deps.secondaryMenuOpen);
  const expandedMetadataRef = useRef(deps.expandedMetadata);
  const capabilityModeRef = useRef(deps.capabilityMode);
  const capabilityStreamingRef = useRef(deps.capabilityStreaming);
  useLayoutEffect(() => {
    modeRef.current = deps.mode;
    cmdResultRef.current = deps.cmdResult;
    queryRef.current = deps.query;
    secondaryMenuOpenRef.current = deps.secondaryMenuOpen;
    expandedMetadataRef.current = deps.expandedMetadata;
    capabilityModeRef.current = deps.capabilityMode;
    capabilityStreamingRef.current = deps.capabilityStreaming;
  });
  return {
    modeRef,
    cmdResultRef,
    queryRef,
    secondaryMenuOpenRef,
    expandedMetadataRef,
    capabilityModeRef,
    capabilityStreamingRef,
  };
}
