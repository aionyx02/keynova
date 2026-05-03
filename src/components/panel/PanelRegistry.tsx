import type React from "react";
import { SettingPanel } from "../SettingPanel";

/** 將後端回傳的 panel name 對應至 React 元件。新增面板只需在此 Record 加一筆。 */
export const PanelRegistry: Record<string, React.ComponentType> = {
  setting: SettingPanel,
};