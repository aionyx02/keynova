use serde::Deserialize;

use crate::models::terminal::TerminalLaunchSpec;

// ── search ───────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub(crate) struct SearchQueryRequest {
    pub query: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub stream: bool,
    #[serde(default)]
    pub request_id: String,
    pub first_batch_limit: Option<usize>,
}

fn default_limit() -> usize {
    10
}

#[derive(Deserialize)]
pub(crate) struct SearchRecordSelectionRequest {
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub path: String,
}

// ── setting ──────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub(crate) struct SettingGetRequest {
    pub key: String,
}

#[derive(Deserialize)]
pub(crate) struct SettingSetRequest {
    pub key: String,
    pub value: String,
}

// ── terminal ─────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub(crate) struct TerminalOpenRequest {
    #[serde(default = "default_rows")]
    pub rows: u16,
    #[serde(default = "default_cols")]
    pub cols: u16,
    pub launch_spec: Option<TerminalLaunchSpec>,
}

fn default_rows() -> u16 {
    24
}
fn default_cols() -> u16 {
    80
}

#[derive(Deserialize)]
pub(crate) struct TerminalSessionRequest {
    pub id: String,
}

#[derive(Deserialize)]
pub(crate) struct TerminalSendRequest {
    pub id: String,
    pub input: String,
}

#[derive(Deserialize)]
pub(crate) struct TerminalResizeRequest {
    pub id: String,
    pub rows: u16,
    pub cols: u16,
}

// ── file ─────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub(crate) struct FileRenameRequest {
    pub path: String,
    pub new_name: String,
    #[serde(default)]
    pub confirm: bool,
}

#[derive(Deserialize)]
pub(crate) struct FileMoveRequest {
    pub path: String,
    pub target_dir: String,
    #[serde(default)]
    pub overwrite: bool,
    #[serde(default)]
    pub confirm: bool,
}

#[derive(Deserialize)]
pub(crate) struct FileDeleteRequest {
    pub path: String,
    #[serde(default)]
    pub confirm: bool,
}

#[derive(Deserialize)]
pub(crate) struct FileHashRequest {
    pub path: String,
    #[serde(default = "default_hash_algo")]
    pub algorithm: String,
}

fn default_hash_algo() -> String {
    "sha256".to_string()
}

#[derive(Deserialize)]
pub(crate) struct FileOpenAsTextRequest {
    pub path: String,
}