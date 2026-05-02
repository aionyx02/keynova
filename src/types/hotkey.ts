export interface HotkeyConfig {
  id: string;
  key: string;
  modifiers: string[];
  action: string;
  enabled: boolean;
}

export interface ConflictInfo {
  conflicting_id: string;
  description: string;
}
