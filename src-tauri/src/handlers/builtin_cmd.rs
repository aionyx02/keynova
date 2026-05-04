use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use crate::core::builtin_command_registry::BuiltinCommand;
use crate::core::config_manager::ConfigManager;
use crate::core::{BuiltinCommandRegistry, CommandHandler, CommandResult};
use crate::models::builtin_command::{BuiltinCommandResult, CommandUiType};

pub struct HelpCommand;

impl BuiltinCommand for HelpCommand {
    fn name(&self) -> &'static str {
        "help"
    }

    fn description(&self) -> &'static str {
        "Show available commands"
    }

    fn execute(&self, _args: &str) -> BuiltinCommandResult {
        BuiltinCommandResult {
            text: String::new(),
            ui_type: CommandUiType::Inline,
        }
    }
}

pub struct SettingCommand;

impl BuiltinCommand for SettingCommand {
    fn name(&self) -> &'static str {
        "setting"
    }

    fn description(&self) -> &'static str {
        "Open or edit settings"
    }

    fn args_hint(&self) -> Option<&'static str> {
        Some("[key] [value]")
    }

    fn execute(&self, _args: &str) -> BuiltinCommandResult {
        BuiltinCommandResult {
            text: String::new(),
            ui_type: CommandUiType::Panel("setting".into()),
        }
    }
}

pub struct ReloadCommand;

impl BuiltinCommand for ReloadCommand {
    fn name(&self) -> &'static str {
        "reload"
    }

    fn description(&self) -> &'static str {
        "Reload config from disk"
    }

    fn execute(&self, _args: &str) -> BuiltinCommandResult {
        BuiltinCommandResult {
            text: "Reload requested".into(),
            ui_type: CommandUiType::Inline,
        }
    }
}

// ─── Phase 3 builtin commands ────────────────────────────────────────────────

pub struct TrCommand;

impl BuiltinCommand for TrCommand {
    fn name(&self) -> &'static str { "tr" }
    fn description(&self) -> &'static str { "Translate text" }
    fn args_hint(&self) -> Option<&'static str> { Some("<src> <dst> <text>  or  default <text>") }
    fn execute(&self, _args: &str) -> BuiltinCommandResult {
        BuiltinCommandResult { text: String::new(), ui_type: CommandUiType::Panel("translation".into()) }
    }
}

pub struct AiCommand;

impl BuiltinCommand for AiCommand {
    fn name(&self) -> &'static str { "ai" }
    fn description(&self) -> &'static str { "Ask AI assistant" }
    fn args_hint(&self) -> Option<&'static str> { Some("<question>") }
    fn execute(&self, _args: &str) -> BuiltinCommandResult {
        BuiltinCommandResult { text: String::new(), ui_type: CommandUiType::Panel("ai".into()) }
    }
}

pub struct NoteCommand;

impl BuiltinCommand for NoteCommand {
    fn name(&self) -> &'static str { "note" }
    fn description(&self) -> &'static str { "Quick notes" }
    fn args_hint(&self) -> Option<&'static str> { Some("[note-name]") }
    fn execute(&self, _args: &str) -> BuiltinCommandResult {
        BuiltinCommandResult { text: String::new(), ui_type: CommandUiType::Panel("note".into()) }
    }
}

pub struct CalCommand;

impl BuiltinCommand for CalCommand {
    fn name(&self) -> &'static str { "cal" }
    fn description(&self) -> &'static str { "Calculator & unit conversion" }
    fn args_hint(&self) -> Option<&'static str> { Some("<expr>  e.g. 2+2, 5 km to m") }
    fn execute(&self, _args: &str) -> BuiltinCommandResult {
        BuiltinCommandResult { text: String::new(), ui_type: CommandUiType::Panel("calculator".into()) }
    }
}

pub struct HistoryCommand;

impl BuiltinCommand for HistoryCommand {
    fn name(&self) -> &'static str { "history" }
    fn description(&self) -> &'static str { "Clipboard history" }
    fn args_hint(&self) -> Option<&'static str> { Some("[search]") }
    fn execute(&self, _args: &str) -> BuiltinCommandResult {
        BuiltinCommandResult { text: String::new(), ui_type: CommandUiType::Panel("history".into()) }
    }
}

pub struct SysCtlCommand;

impl BuiltinCommand for SysCtlCommand {
    fn name(&self) -> &'static str { "system" }
    fn description(&self) -> &'static str { "System control: volume, brightness, wifi" }
    fn args_hint(&self) -> Option<&'static str> { Some("volume|brightness|wifi [value]") }
    fn execute(&self, _args: &str) -> BuiltinCommandResult {
        BuiltinCommandResult { text: String::new(), ui_type: CommandUiType::Panel("system".into()) }
    }
}

pub struct DownCommand;

impl BuiltinCommand for DownCommand {
    fn name(&self) -> &'static str {
        "down"
    }

    fn description(&self) -> &'static str {
        "Gracefully quit Keynova"
    }

    fn execute(&self, _args: &str) -> BuiltinCommandResult {
        BuiltinCommandResult {
            text: "Shutdown requested".into(),
            ui_type: CommandUiType::Inline,
        }
    }
}

pub struct BuiltinCmdHandler {
    registry: Arc<Mutex<BuiltinCommandRegistry>>,
    config: Arc<Mutex<ConfigManager>>,
}

impl BuiltinCmdHandler {
    pub fn new(
        registry: Arc<Mutex<BuiltinCommandRegistry>>,
        config: Arc<Mutex<ConfigManager>>,
    ) -> Self {
        Self { registry, config }
    }
}

impl CommandHandler for BuiltinCmdHandler {
    fn namespace(&self) -> &'static str {
        "cmd"
    }

    fn execute(&self, command: &str, payload: Value) -> CommandResult {
        match command {
            "list" => {
                let reg = self.registry.lock().map_err(|e| e.to_string())?;
                Ok(json!(reg.list()))
            }
            "run" => {
                let name = payload
                    .get("name")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing 'name' field".to_string())?;
                let args = payload
                    .get("args")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .trim();

                if name == "setting" && !args.is_empty() {
                    let mut parts = args.splitn(2, ' ');
                    let key = parts.next().unwrap_or("").trim();
                    let value_opt = parts.next().map(str::trim).filter(|s| !s.is_empty());
                    return match value_opt {
                        Some(value) => {
                            let mut cfg = self.config.lock().map_err(|e| e.to_string())?;
                            cfg.set(key, value).map_err(|e| e.to_string())?;
                            Ok(json!(BuiltinCommandResult {
                                text: format!("✓ {} = {}", key, value),
                                ui_type: CommandUiType::Inline,
                            }))
                        }
                        None => {
                            let cfg = self.config.lock().map_err(|e| e.to_string())?;
                            let current = cfg.get(key).unwrap_or_else(|| "(not set)".to_string());
                            Ok(json!(BuiltinCommandResult {
                                text: format!("{} = {}", key, current),
                                ui_type: CommandUiType::Inline,
                            }))
                        }
                    };
                }

                if name == "help" {
                    let list = {
                        let reg = self.registry.lock().map_err(|e| e.to_string())?;
                        reg.list()
                    };
                    let text = list
                        .iter()
                        .map(|meta| format!("/{} - {}", meta.name, meta.description))
                        .collect::<Vec<_>>()
                        .join("\n");
                    return Ok(json!(BuiltinCommandResult {
                        text,
                        ui_type: CommandUiType::Inline,
                    }));
                }

                let result = {
                    let reg = self.registry.lock().map_err(|e| e.to_string())?;
                    reg.run(name, args)
                };
                result
                    .map(|result| json!(result))
                    .ok_or_else(|| format!("unknown command '/{name}'"))
            }
            "suggest_args" => {
                let name = payload
                    .get("name")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing 'name'".to_string())?;
                let partial = payload
                    .get("partial")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_lowercase();
                if name == "setting" {
                    let cfg = self.config.lock().map_err(|e| e.to_string())?;
                    let keys: Vec<String> = cfg
                        .list_all()
                        .into_iter()
                        .filter(|(k, _)| partial.is_empty() || k.starts_with(&partial))
                        .map(|(k, _)| k)
                        .collect();
                    return Ok(json!(keys));
                }
                Ok(json!(Vec::<String>::new()))
            }
            _ => Err(format!("unknown cmd command '{command}'")),
        }
    }
}
