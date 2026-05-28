// REF.2.P3 — Secondary-action menu state.
//
// Owns the five state pieces that move together when the user presses
// `→` / `Tab` on a search result to expand the secondary-action menu:
//
//   - `secondaryMenuOpen`  — whether the menu is rendered
//   - `menuFocusedIndex`   — keyboard cursor inside the menu
//   - `expandedMetadata`   — whether the metadata accordion is open
//   - `pendingConfirm`     — secondary action awaiting its second Enter
//                             (rename / move / delete two-phase gate)
//   - `inlineInput`        — rename / move inline-edit input value
//
// `closeSecondaryMenu()` is the universal teardown that several call sites
// invoke (ESC handler, post-action success, result selection change, etc.).

import { useCallback, useState } from "react";

import type { SecondaryActionId } from "../../../utils/secondaryActions";

export type SecondaryInlineInput = {
  for: "rename" | "move";
  value: string;
};

export interface UseSecondaryMenu {
  secondaryMenuOpen: boolean;
  setSecondaryMenuOpen: React.Dispatch<React.SetStateAction<boolean>>;
  menuFocusedIndex: number;
  setMenuFocusedIndex: React.Dispatch<React.SetStateAction<number>>;
  expandedMetadata: boolean;
  setExpandedMetadata: React.Dispatch<React.SetStateAction<boolean>>;
  pendingConfirm: SecondaryActionId | null;
  setPendingConfirm: React.Dispatch<React.SetStateAction<SecondaryActionId | null>>;
  inlineInput: SecondaryInlineInput | null;
  setInlineInput: React.Dispatch<React.SetStateAction<SecondaryInlineInput | null>>;
  /** Reset menu / focus index / pending confirm / inline-input (leaves expanded metadata alone). */
  closeSecondaryMenu: () => void;
}

export function useSecondaryMenu(): UseSecondaryMenu {
  const [secondaryMenuOpen, setSecondaryMenuOpen] = useState(false);
  const [menuFocusedIndex, setMenuFocusedIndex] = useState(0);
  const [expandedMetadata, setExpandedMetadata] = useState(false);
  const [pendingConfirm, setPendingConfirm] = useState<SecondaryActionId | null>(null);
  const [inlineInput, setInlineInput] = useState<SecondaryInlineInput | null>(null);

  const closeSecondaryMenu = useCallback(() => {
    setSecondaryMenuOpen(false);
    setMenuFocusedIndex(0);
    setPendingConfirm(null);
    setInlineInput(null);
  }, []);

  return {
    secondaryMenuOpen,
    setSecondaryMenuOpen,
    menuFocusedIndex,
    setMenuFocusedIndex,
    expandedMetadata,
    setExpandedMetadata,
    pendingConfirm,
    setPendingConfirm,
    inlineInput,
    setInlineInput,
    closeSecondaryMenu,
  };
}
