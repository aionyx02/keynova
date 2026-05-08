use serde::{Deserialize, Serialize};

use crate::models::terminal::TerminalLaunchSpec;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum CommandUiType {
    Inline,
    Panel(String),
    Terminal(TerminalLaunchSpec),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuiltinCommandResult {
    pub text: String,
    pub ui_type: CommandUiType,
}
