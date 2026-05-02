use serde::{Deserialize, Serialize};

/// 系統已安裝應用程式的基本資訊。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInfo {
    pub name: String,
    pub path: String,
    /// Base64 編碼的圖示資料（PNG），None 代表使用預設圖示。
    pub icon_data: Option<String>,
    /// 歷史啟動次數，用於排序搜尋結果。
    pub launch_count: u32,
}
