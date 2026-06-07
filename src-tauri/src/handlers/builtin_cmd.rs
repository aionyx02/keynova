use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use crate::core::builtin_command_registry::BuiltinCommand;
use crate::core::config_manager::ConfigManager;
use crate::core::{BuiltinCommandRegistry, CommandHandler, CommandResult};
use crate::managers::search_manager::SearchManager;
use crate::models::builtin_command::{BuiltinCommandResult, CommandUiType};
use crate::models::settings_schema::is_sensitive_key;

mod note;
pub use note::NoteCommand;

/// Builtin slash-command name → `features.*` flag that gates it. Shared so the
/// command **search provider** can hide a disabled feature's command (matching
/// executability) and the handler can refuse to run it. Single source of truth
/// for "which command belongs to a gateable feature".
pub(crate) const COMMAND_FEATURE_GUARDS: &[(&str, &str)] = &[
    ("ai", "features.ai"),
    ("tr", "features.translation"),
    ("note", "features.notes"),
    ("history", "features.history"),
    ("cal", "features.calculator"),
    ("system", "features.system"),
    ("system_monitoring", "features.system"),
];

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

pub struct OnboardCommand;

impl BuiltinCommand for OnboardCommand {
    fn name(&self) -> &'static str {
        "onboard"
    }

    fn description(&self) -> &'static str {
        "Replay the onboarding tour"
    }

    fn execute(&self, _args: &str) -> BuiltinCommandResult {
        // Frontend intercepts `/onboard` before the result is rendered (clears
        // localStorage flag + reopens overlay). This inline text is the
        // fallback for any code path that reads the BuiltinCommandResult.
        BuiltinCommandResult {
            text: "Replaying onboarding tour…".into(),
            ui_type: CommandUiType::Inline,
        }
    }
}

// ─── Phase 3 builtin commands ────────────────────────────────────────────────

pub struct TrCommand;

impl BuiltinCommand for TrCommand {
    fn name(&self) -> &'static str {
        "tr"
    }
    fn description(&self) -> &'static str {
        "Translate text"
    }
    fn args_hint(&self) -> Option<&'static str> {
        Some("<src> <dst> <text>  or  default <text>")
    }
    fn execute(&self, args: &str) -> BuiltinCommandResult {
        BuiltinCommandResult {
            text: args.to_string(),
            ui_type: CommandUiType::Panel("translation".into()),
        }
    }
}

// REF.6.B follow-up — `AiCommand` removed. The chat-first `/ai` panel does
// not match the prefix-keyword inline AI flow (`explain <q>` /
// `summarize <text>`). Reinstate when chat returns (see ADR-0029 §2.5).

pub struct ModelCommand;

impl BuiltinCommand for ModelCommand {
    fn name(&self) -> &'static str {
        "model"
    }
    fn description(&self) -> &'static str {
        "Manage AI models: switch, download, or remove"
    }
    fn execute(&self, _args: &str) -> BuiltinCommandResult {
        BuiltinCommandResult {
            text: String::new(),
            ui_type: CommandUiType::Panel("model".into()),
        }
    }
}

fn setting_assignment_result_text(key: &str, value: &str) -> String {
    if is_sensitive_key(key) && !value.is_empty() {
        format!("✓ {key} updated locally")
    } else {
        format!("✓ {key} = {value}")
    }
}

fn setting_lookup_result_text(key: &str, current: Option<String>) -> String {
    let value = match current {
        Some(value) if is_sensitive_key(key) && !value.is_empty() => "********".to_string(),
        Some(value) => value,
        None => "(not set)".to_string(),
    };
    format!("{key} = {value}")
}

pub struct CalCommand;

impl BuiltinCommand for CalCommand {
    fn name(&self) -> &'static str {
        "cal"
    }
    fn description(&self) -> &'static str {
        "Calculator & unit conversion"
    }
    fn args_hint(&self) -> Option<&'static str> {
        Some("<expr>  e.g. 2+2, 5 km to m")
    }
    fn execute(&self, _args: &str) -> BuiltinCommandResult {
        BuiltinCommandResult {
            text: String::new(),
            ui_type: CommandUiType::Panel("calculator".into()),
        }
    }
}

pub struct HistoryCommand;

impl BuiltinCommand for HistoryCommand {
    fn name(&self) -> &'static str {
        "history"
    }
    fn description(&self) -> &'static str {
        "Clipboard history"
    }
    fn args_hint(&self) -> Option<&'static str> {
        Some("[search]")
    }
    fn execute(&self, _args: &str) -> BuiltinCommandResult {
        BuiltinCommandResult {
            text: String::new(),
            ui_type: CommandUiType::Panel("history".into()),
        }
    }
}

pub struct SysCtlCommand;

impl BuiltinCommand for SysCtlCommand {
    fn name(&self) -> &'static str {
        "system"
    }
    fn description(&self) -> &'static str {
        "System control: volume, brightness, wifi"
    }
    fn args_hint(&self) -> Option<&'static str> {
        Some("volume|brightness|wifi [value]")
    }
    fn execute(&self, _args: &str) -> BuiltinCommandResult {
        BuiltinCommandResult {
            text: String::new(),
            ui_type: CommandUiType::Panel("system".into()),
        }
    }
}

pub struct SysMonitorCommand;

impl BuiltinCommand for SysMonitorCommand {
    fn name(&self) -> &'static str {
        "system_monitoring"
    }
    fn description(&self) -> &'static str {
        "CPU, RAM, Disk, Network & Process monitor"
    }
    fn execute(&self, _args: &str) -> BuiltinCommandResult {
        BuiltinCommandResult {
            text: String::new(),
            ui_type: CommandUiType::Panel("system_monitoring".into()),
        }
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

pub struct UpdateCommand;

impl BuiltinCommand for UpdateCommand {
    fn name(&self) -> &'static str {
        "update"
    }

    fn description(&self) -> &'static str {
        "Check for app updates"
    }

    fn execute(&self, _args: &str) -> BuiltinCommandResult {
        // The updater check needs the Tauri AppHandle, which command handlers
        // don't have, so the frontend intercepts `/update` (like `/onboard`) and
        // drives the plugin. This inline fallback covers any non-UI path.
        BuiltinCommandResult {
            text: String::new(),
            ui_type: CommandUiType::Inline,
        }
    }
}

pub struct DiagCommand;

impl BuiltinCommand for DiagCommand {
    fn name(&self) -> &'static str {
        "diag"
    }

    fn description(&self) -> &'static str {
        "Export a redacted diagnostics bundle"
    }

    fn execute(&self, _args: &str) -> BuiltinCommandResult {
        // The real bundle needs config + local paths + preflight, so it is
        // assembled by `BuiltinCmdHandler`. This inline fallback covers any direct
        // registry call path.
        BuiltinCommandResult {
            text: String::new(),
            ui_type: CommandUiType::Inline,
        }
    }
}

pub struct RebuildSearchIndexCommand;

impl BuiltinCommand for RebuildSearchIndexCommand {
    fn name(&self) -> &'static str {
        "rebuild_search_index"
    }

    fn description(&self) -> &'static str {
        "Rebuild the local search index"
    }

    fn execute(&self, _args: &str) -> BuiltinCommandResult {
        BuiltinCommandResult {
            text: "Search index rebuild requested".into(),
            ui_type: CommandUiType::Inline,
        }
    }
}

pub struct BuiltinCmdHandler {
    registry: Arc<Mutex<BuiltinCommandRegistry>>,
    config: Arc<Mutex<ConfigManager>>,
    search_manager: Arc<Mutex<SearchManager>>,
}

impl BuiltinCmdHandler {
    pub fn new(
        registry: Arc<Mutex<BuiltinCommandRegistry>>,
        config: Arc<Mutex<ConfigManager>>,
        search_manager: Arc<Mutex<SearchManager>>,
    ) -> Self {
        Self {
            registry,
            config,
            search_manager,
        }
    }

    /// Gather the redacted diagnostics inputs (config, local data paths, preflight
    /// snapshot) and assemble the report. All path I/O is metadata-only; secrets
    /// come pre-masked from `list_all_redacted()`.
    fn build_diagnostics_report(&self) -> crate::core::diagnostics::DiagnosticsReport {
        use crate::core::diagnostics::{
            build_report, path_size, DiagnosticsInputs, PreflightFacts, ResolvedPath,
        };
        use crate::platform_dirs::{keynova_config_dir, keynova_data_dir};

        let redacted_config = self
            .config
            .lock()
            .map(|cfg| cfg.list_all_redacted())
            .unwrap_or_default();

        let config_dir = keynova_config_dir();
        let data_dir = keynova_data_dir();
        let candidates = [
            ("config.toml", config_dir.join("config.toml")),
            ("knowledge.db", data_dir.join("knowledge.db")),
            ("notes/", data_dir.join("notes")),
            ("search index", data_dir.join("search").join("tantivy")),
            (
                "preflight snapshot",
                data_dir.join("bootstrap").join("preflight-v1.json"),
            ),
        ];
        let paths = candidates
            .into_iter()
            .map(|(label, path)| {
                let exists = path.exists();
                let size_bytes = if exists { path_size(&path) } else { 0 };
                ResolvedPath {
                    label: label.to_string(),
                    path,
                    exists,
                    size_bytes,
                }
            })
            .collect();

        let preflight =
            crate::core::startup_preflight::read_snapshot_from_disk().map(|s| PreflightFacts {
                status: s.status,
                source_mode: s.source_mode,
                ollama_reachable: s.model.ollama_reachable,
                local_model_count: s.model.local_models.len(),
                generated_at: s.generated_at,
            });

        let last_crash =
            crate::core::crash_log::read_last_crash(&data_dir.join("crash.log"));

        build_report(DiagnosticsInputs {
            version: env!("CARGO_PKG_VERSION").to_string(),
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            home_dir: dirs::home_dir(),
            redacted_config,
            paths,
            preflight,
            last_crash,
        })
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

                // Feature guard: disabled features return a friendly message instead of executing.
                for &(cmd_name, feature_key) in COMMAND_FEATURE_GUARDS {
                    if name == cmd_name {
                        let cfg = self.config.lock().map_err(|e| e.to_string())?;
                        let enabled = cfg
                            .get(feature_key)
                            .as_deref()
                            .map(|v| !v.eq_ignore_ascii_case("false"))
                            .unwrap_or(true);
                        if !enabled {
                            return Ok(json!(BuiltinCommandResult {
                                text: format!(
                                    "/{name} 功能已停用。請前往 /setting → Features 開啟。"
                                ),
                                ui_type: CommandUiType::Inline,
                            }));
                        }
                        break;
                    }
                }

                if name == "setting" && !args.is_empty() {
                    let mut parts = args.splitn(2, ' ');
                    let key = parts.next().unwrap_or("").trim();
                    let value_opt = parts.next().map(str::trim).filter(|s| !s.is_empty());
                    return match value_opt {
                        Some(value) => {
                            let mut cfg = self.config.lock().map_err(|e| e.to_string())?;
                            cfg.set_user_value(key, value).map_err(|e| e.to_string())?;
                            Ok(json!(BuiltinCommandResult {
                                text: setting_assignment_result_text(key, value),
                                ui_type: CommandUiType::Inline,
                            }))
                        }
                        None => {
                            let cfg = self.config.lock().map_err(|e| e.to_string())?;
                            Ok(json!(BuiltinCommandResult {
                                text: setting_lookup_result_text(key, cfg.get(key)),
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

                if name == "diag" {
                    let report = self.build_diagnostics_report();
                    return Ok(json!(BuiltinCommandResult {
                        text: crate::core::diagnostics::render_text(&report),
                        ui_type: CommandUiType::Inline,
                    }));
                }

                if name == "rebuild_search_index" {
                    let status = self
                        .search_manager
                        .lock()
                        .map_err(|e| e.to_string())?
                        .rebuild_index();
                    return Ok(json!(BuiltinCommandResult {
                        text: status.message,
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
                    let mut keys: Vec<String> = cfg
                        .schema()
                        .into_iter()
                        .map(|schema| schema.key.to_string())
                        .filter(|key| partial.is_empty() || key.starts_with(&partial))
                        .collect();
                    if keys.is_empty() {
                        keys = cfg
                            .list_all()
                            .into_iter()
                            .filter(|(k, _)| partial.is_empty() || k.starts_with(&partial))
                            .map(|(k, _)| k)
                            .collect();
                    }
                    return Ok(json!(keys));
                }
                Ok(json!(Vec::<String>::new()))
            }
            _ => Err(format!("unknown cmd command '{command}'")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sensitive_setting_assignment_result_redacts_value() {
        assert_eq!(
            setting_assignment_result_text("translation.api_key", "secret-value"),
            "✓ translation.api_key updated locally"
        );
    }

    #[test]
    fn sensitive_setting_lookup_result_masks_value() {
        assert_eq!(
            setting_lookup_result_text("translation.api_key", Some("secret-value".into())),
            "translation.api_key = ********"
        );
    }
}
