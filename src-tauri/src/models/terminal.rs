use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TerminalEnvVar {
    pub key: String,
    pub value: String,
}

/// Direct process launch request for a terminal-backed command result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TerminalLaunchSpec {
    pub launch_id: String,
    pub program: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub env: Vec<TerminalEnvVar>,
    #[serde(default)]
    pub editor: bool,
}

/// 終端 Session 的狀態。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TerminalStatus {
    Running,
    Exited,
}

/// 單個終端 Session 的描述資料。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalSession {
    pub id: String,
    pub rows: u16,
    pub cols: u16,
    pub status: TerminalStatus,
}
