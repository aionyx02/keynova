use serde::{Deserialize, Serialize};

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
