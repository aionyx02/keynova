//! `note` builtin command — opens the in-app notes panel.
//!
//! REF.8: the LazyVim / external-editor launch path was removed with the nvim
//! feature. `note` now always opens the built-in note panel.

use crate::core::builtin_command_registry::BuiltinCommand;
use crate::models::builtin_command::{BuiltinCommandResult, CommandUiType};

pub struct NoteCommand;

impl NoteCommand {
    pub fn new() -> Self {
        Self
    }
}

impl Default for NoteCommand {
    fn default() -> Self {
        Self::new()
    }
}

impl BuiltinCommand for NoteCommand {
    fn name(&self) -> &'static str {
        "note"
    }
    fn description(&self) -> &'static str {
        "Quick notes"
    }
    fn execute(&self, _args: &str) -> BuiltinCommandResult {
        BuiltinCommandResult {
            text: String::new(),
            ui_type: CommandUiType::Panel("note".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn note_opens_builtin_panel() {
        let result = NoteCommand::new().execute("");
        assert!(matches!(result.ui_type, CommandUiType::Panel(ref name) if name == "note"));
        assert!(result.text.is_empty());
    }

    #[test]
    fn note_ignores_legacy_lazyvim_args() {
        let result = NoteCommand::new().execute("lazyvim project plan");
        assert!(matches!(result.ui_type, CommandUiType::Panel(ref name) if name == "note"));
    }
}
