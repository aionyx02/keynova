use serde::{Deserialize, Serialize};

/// 搜尋結果的種類（應用程式、檔案 或 資料夾）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ResultKind {
    App,
    File,
    Folder,
    Command,
    Note,
    History,
    Model,
    /// MEM.1.C — a stored personal memory surfaced in search (gated by
    /// `features.ai`). Produced by the memory provider, never by file search.
    Memory,
}

/// 統一搜尋結果，可來自 App 快取或 Everything IPC。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub kind: ResultKind,
    pub name: String,
    pub path: String,
    pub score: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_kind_serializes_snake_case() {
        let json = serde_json::to_string(&ResultKind::Memory).unwrap();
        assert_eq!(json, "\"memory\"");
        let back: ResultKind = serde_json::from_str("\"memory\"").unwrap();
        assert_eq!(back, ResultKind::Memory);
    }
}
