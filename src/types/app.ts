export interface AppInfo {
  name: string;
  path: string;
  /** Base64 PNG，null 代表使用預設圖示 */
  icon_data: string | null;
  launch_count: number;
}
