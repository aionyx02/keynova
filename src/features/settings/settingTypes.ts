export interface SettingEntry {
  key: string;
  value: string;
  sensitive?: boolean;
}

export interface SettingSchema {
  key: string;
  section: string;
  label: string;
  value_type: "string" | "integer" | "boolean" | "hotkey" | "secret";
  default_value: string;
  sensitive: boolean;
  options: string[];
}
