use serde::{Deserialize, Serialize};

/// 搜尋結果的種類（應用程式、檔案 或 資料夾）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ResultKind {
    App,
    File,
    Folder,
}

/// 統一搜尋結果，可來自 App 快取或 Everything IPC。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub kind: ResultKind,
    pub name: String,
    pub path: String,
    pub score: i64,
}
