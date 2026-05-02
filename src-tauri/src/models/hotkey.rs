use serde::{Deserialize, Serialize};

/// 全局快捷鍵的設定結構。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotkeyConfig {
    /// 唯一識別符（例如 "app_launcher"）。
    pub id: String,
    /// 主鍵（例如 "K", "T", "P"）。
    pub key: String,
    /// 修飾鍵列表（例如 ["Win"]、["Ctrl", "Alt"]）。
    pub modifiers: Vec<String>,
    /// 觸發的動作識別符。
    pub action: String,
    pub enabled: bool,
}

/// 快捷鍵衝突資訊。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictInfo {
    pub conflicting_id: String,
    pub description: String,
}
