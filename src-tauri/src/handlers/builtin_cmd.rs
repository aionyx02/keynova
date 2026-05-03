use std::sync::{Arc, Mutex};
use serde_json::{json, Value};

use crate::core::{BuiltinCommandRegistry, CommandHandler, CommandResult};
use crate::core::config_manager::ConfigManager;
use crate::models::builtin_command::{BuiltinCommandResult, CommandUiType};
use crate::core::builtin_command_registry::BuiltinCommand;

// ─── Concrete commands ───────────────────────────────────────────────────────

/// /help — 元資料僅供 cmd.list 顯示；實際執行由 BuiltinCmdHandler 特殊處理以避免鎖重入。
pub struct HelpCommand;

impl BuiltinCommand for HelpCommand {
    fn name(&self) -> &'static str { "help" }
    fn description(&self) -> &'static str { "顯示所有可用指令" }
    fn execute(&self, _args: &str) -> BuiltinCommandResult {
        BuiltinCommandResult { text: String::new(), ui_type: CommandUiType::Inline }
    }
}

/// /setting — 通知前端開啟設定面板。
pub struct SettingCommand;

impl BuiltinCommand for SettingCommand {
    fn name(&self) -> &'static str { "setting" }
    fn description(&self) -> &'static str { "開啟設定面板" }
    fn execute(&self, _args: &str) -> BuiltinCommandResult {
        BuiltinCommandResult {
            text: String::new(),
            ui_type: CommandUiType::Panel("setting".into()),
        }
    }
}

// ─── Handler ─────────────────────────────────────────────────────────────────

/// 處理 `cmd.*` 命名空間的 IPC 請求。
pub struct BuiltinCmdHandler {
    registry: Arc<Mutex<BuiltinCommandRegistry>>,
    config: Arc<Mutex<ConfigManager>>,
}

impl BuiltinCmdHandler {
    pub fn new(registry: Arc<Mutex<BuiltinCommandRegistry>>, config: Arc<Mutex<ConfigManager>>) -> Self {
        Self { registry, config }
    }
}

impl CommandHandler for BuiltinCmdHandler {
    fn namespace(&self) -> &'static str { "cmd" }

    fn execute(&self, command: &str, payload: Value) -> CommandResult {
        match command {
            "list" => {
                let reg = self.registry.lock().map_err(|e| e.to_string())?;
                Ok(json!(reg.list()))
            }
            "run" => {
                let name = payload.get("name")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing 'name' field".to_string())?;
                let args = payload.get("args")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .trim();

                // /setting <key> [value] — Minecraft 風格內聯設定
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
                            let current = cfg.get(key).unwrap_or_else(|| "(找不到此設定)".to_string());
                            Ok(json!(BuiltinCommandResult {
                                text: format!("{} = {}", key, current),
                                ui_type: CommandUiType::Inline,
                            }))
                        }
                    };
                }

                // /help 在 handler 內特殊處理，避免 Mutex 鎖重入
                if name == "help" {
                    let list = {
                        let reg = self.registry.lock().map_err(|e| e.to_string())?;
                        reg.list()
                    };
                    let text = list.iter()
                        .map(|m| format!("/{} — {}", m.name, m.description))
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
                    .map(|r| json!(r))
                    .ok_or_else(|| format!("unknown command '/{name}'"))
            }
            _ => Err(format!("unknown cmd command '{command}'")),
        }
    }
}