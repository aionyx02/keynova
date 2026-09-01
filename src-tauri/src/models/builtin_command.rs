use serde::{Deserialize, Serialize};

use crate::models::terminal::TerminalLaunchSpec;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum CommandUiType {
    Inline,
    Panel(String),
    Terminal(TerminalLaunchSpec),
    /// Renders nowhere in the palette: the frontend opens the named OS window
    /// instead. The string is a window identity (`"settings"`), not a route —
    /// the frontend maps it to a specific command and ignores anything it does
    /// not recognise, so a newer backend cannot make an older UI open something
    /// arbitrary.
    Window(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuiltinCommandResult {
    pub text: String,
    pub ui_type: CommandUiType,
}
