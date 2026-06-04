use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use crate::app::feature_registry::{AssemblyCtx, FeatureRegistrar, FeatureSpec};
use crate::core::config_manager::ConfigManager;
use crate::core::{CommandHandler, CommandResult};
use crate::managers::{
    learning_material_manager::LearningMaterialManager, note_manager::NoteManager,
};

/// Handles `learning_material.*` IPC commands.
///
/// File-reading commands require `agent.local_context.enabled = true` plus
/// approved roots. Export commands write to NoteManager-managed storage.
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

/// DECOUP.3 (ADR-0044): self-register learning material. Uses shared `config` +
/// `note_manager` from ctx (note storage is shared with notes/search/agent).
pub fn register(reg: &mut FeatureRegistrar, ctx: &AssemblyCtx) {
    reg.handler(Arc::new(LearningMaterialHandler::new(
        Arc::clone(&ctx.config),
        Arc::clone(&ctx.note_manager),
    )));
    reg.spec(FeatureSpec {
        namespace: "learning_material",
        flag_key: None,
    });
}

impl CommandHandler for LearningMaterialHandler {
    fn namespace(&self) -> &'static str {
        "learning_material"
    }

    fn execute(&self, command: &str, payload: Value) -> CommandResult {
        match command {
            "scan" => {
                let roots = parse_roots(&payload);

                let mgr = self.make_manager()?;
                let report = mgr.scan(&roots)?;
                serde_json::to_value(report).map_err(|e| e.to_string())
            }

            "preview" => {
                let path = payload
                    .get("path")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing 'path'".to_string())?;
                let roots = parse_roots(&payload);
                let mgr = self.make_manager()?;
                let (canonical_path, preview) =
                    mgr.preview_file_with_roots(Path::new(path), &roots)?;
                Ok(json!({ "path": canonical_path.display().to_string(), "preview": preview }))
            }

            "export_note" => {
                let title = payload
                    .get("title")
                    .and_then(Value::as_str)
                    .unwrap_or("Learning Review")
                    .trim()
                    .to_string();

                let report: crate::models::learning_material::ReviewReport =
                    serde_json::from_value(payload.get("report").cloned().unwrap_or(Value::Null))
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

                let note_mgr = self.note_manager.lock().map_err(|e| e.to_string())?;
                let canonical_target = resolve_markdown_export_target(&note_mgr, path)?;

                let report: crate::models::learning_material::ReviewReport =
                    serde_json::from_value(payload.get("report").cloned().unwrap_or(Value::Null))
                        .map_err(|e| format!("invalid report payload: {e}"))?;

                let content = report.to_markdown();
                std::fs::write(&canonical_target, content).map_err(|e| e.to_string())?;
                Ok(json!({ "ok": true, "path": canonical_target.display().to_string() }))
            }

            _ => Err(format!("unknown learning_material command '{command}'")),
        }
    }
}

fn parse_roots(payload: &Value) -> Vec<PathBuf> {
    payload
        .get("roots")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .map(PathBuf::from)
                .collect()
        })
        .unwrap_or_default()
}

fn resolve_markdown_export_target(
    note_mgr: &NoteManager,
    raw_path: &str,
) -> Result<PathBuf, String> {
    let requested = PathBuf::from(raw_path);
    let extension = requested
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !matches!(extension.as_str(), "md" | "markdown") {
        return Err("export_markdown path must end with .md or .markdown".into());
    }

    let notes_root = note_mgr.notes_root();
    std::fs::create_dir_all(&notes_root).map_err(|e| e.to_string())?;
    let canonical_root = notes_root
        .canonicalize()
        .map_err(|e| format!("invalid notes root: {e}"))?;

    let target = if requested.is_absolute() {
        requested
    } else {
        notes_root.join(requested)
    };
    let filename = target
        .file_name()
        .ok_or_else(|| "path must end with a filename".to_string())?;
    let parent = target.parent().unwrap_or(&notes_root);
    let canonical_parent = parent
        .canonicalize()
        .map_err(|e| format!("invalid export path: {e}"))?;

    if !canonical_parent.starts_with(&canonical_root) {
        return Err(format!(
            "export_markdown path must stay inside notes root '{}'",
            canonical_root.display()
        ));
    }

    let canonical_target = canonical_parent.join(filename);
    if canonical_target.exists() {
        let existing = canonical_target
            .canonicalize()
            .map_err(|e| format!("invalid export target: {e}"))?;
        if !existing.starts_with(&canonical_root) {
            return Err("export_markdown target resolves outside notes root".into());
        }
        if existing.is_dir() {
            return Err("export_markdown target must be a file".into());
        }
        return Ok(existing);
    }

    Ok(canonical_target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn mk_tmp(suffix: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("keynova-lm-handler-{suffix}-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("create tmp dir");
        dir.canonicalize().expect("canonicalize tmp dir")
    }

    #[test]
    fn markdown_export_relative_path_resolves_inside_notes_root() {
        let root = mk_tmp("notes-root");
        let note_mgr = NoteManager::new(Some(root.display().to_string()));

        let target = resolve_markdown_export_target(&note_mgr, "review.md")
            .expect("relative markdown export target");

        assert!(target.starts_with(&root));
        assert_eq!(
            target.file_name().and_then(|v| v.to_str()),
            Some("review.md")
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn markdown_export_rejects_absolute_path_outside_notes_root() {
        let root = mk_tmp("notes-root");
        let outside = mk_tmp("outside");
        let note_mgr = NoteManager::new(Some(root.display().to_string()));

        let error = resolve_markdown_export_target(
            &note_mgr,
            &outside.join("review.md").display().to_string(),
        )
        .expect_err("outside target must be rejected");

        assert!(error.contains("inside notes root"));
        let _ = std::fs::remove_dir_all(root);
        let _ = std::fs::remove_dir_all(outside);
    }

    #[test]
    fn markdown_export_rejects_non_markdown_extension() {
        let root = mk_tmp("notes-root");
        let note_mgr = NoteManager::new(Some(root.display().to_string()));

        let error = resolve_markdown_export_target(&note_mgr, "review.txt")
            .expect_err("non-markdown target must be rejected");

        assert!(error.contains(".md"));
        let _ = std::fs::remove_dir_all(root);
    }
}
