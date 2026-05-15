use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use crate::core::config_manager::ConfigManager;
use crate::core::{CommandHandler, CommandResult};
use crate::managers::{
    learning_material_manager::LearningMaterialManager, note_manager::NoteManager,
};

/// Handles `learning_material.*` IPC commands.
///
/// All scan commands require `agent.local_context.enabled = true` in settings.
/// Export commands write to the user's NoteManager storage (approval enforced at call site).
pub struct LearningMaterialHandler {
    config: Arc<Mutex<ConfigManager>>,
    note_manager: Arc<Mutex<NoteManager>>,
}

impl LearningMaterialHandler {
    pub fn new(config: Arc<Mutex<ConfigManager>>, note_manager: Arc<Mutex<NoteManager>>) -> Self {
        Self {
            config,
            note_manager,
        }
    }

    fn make_manager(&self) -> Result<LearningMaterialManager, String> {
        let config = self.config.lock().map_err(|e| e.to_string())?;
        Ok(LearningMaterialManager::from_config(&config))
    }
}

impl CommandHandler for LearningMaterialHandler {
    fn namespace(&self) -> &'static str {
        "learning_material"
    }

    fn execute(&self, command: &str, payload: Value) -> CommandResult {
        match command {
            "scan" => {
                let roots: Vec<PathBuf> = payload
                    .get("roots")
                    .and_then(Value::as_array)
                    .map(|arr| {
                        arr.iter()
                            .filter_map(Value::as_str)
                            .map(PathBuf::from)
                            .collect()
                    })
                    .unwrap_or_default();

                let mgr = self.make_manager()?;
                let report = mgr.scan(&roots)?;
                serde_json::to_value(report).map_err(|e| e.to_string())
            }

            "preview" => {
                let path = payload
                    .get("path")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing 'path'".to_string())?;
                let mgr = self.make_manager()?;
                let preview = mgr.preview_file(std::path::Path::new(path));
                Ok(json!({ "path": path, "preview": preview }))
            }

            "export_note" => {
                let title = payload
                    .get("title")
                    .and_then(Value::as_str)
                    .unwrap_or("Learning Review")
                    .trim()
                    .to_string();

                let report: crate::models::learning_material::ReviewReport =
                    serde_json::from_value(
                        payload.get("report").cloned().unwrap_or(Value::Null),
                    )
                    .map_err(|e| format!("invalid report payload: {e}"))?;

                let content = report.to_markdown();
                let note_mgr = self.note_manager.lock().map_err(|e| e.to_string())?;
                // Use save (auto-creates) to allow overwriting a prior draft.
                note_mgr.save(&title, &content)?;
                Ok(json!({ "ok": true, "name": title }))
            }

            "export_markdown" => {
                let path = payload
                    .get("path")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing 'path'".to_string())?;

                // Security: canonicalize the target path's parent and verify it's writable.
                let target = PathBuf::from(path);
                let parent = target.parent().unwrap_or(std::path::Path::new("."));
                let canonical_parent = parent
                    .canonicalize()
                    .map_err(|e| format!("invalid export path: {e}"))?;
                let canonical_target = canonical_parent.join(
                    target
                        .file_name()
                        .ok_or_else(|| "path must end with a filename".to_string())?,
                );

                let report: crate::models::learning_material::ReviewReport =
                    serde_json::from_value(
                        payload.get("report").cloned().unwrap_or(Value::Null),
                    )
                    .map_err(|e| format!("invalid report payload: {e}"))?;

                let content = report.to_markdown();
                std::fs::write(&canonical_target, content).map_err(|e| e.to_string())?;
                Ok(json!({ "ok": true, "path": canonical_target.display().to_string() }))
            }

            _ => Err(format!("unknown learning_material command '{command}'")),
        }
    }
}