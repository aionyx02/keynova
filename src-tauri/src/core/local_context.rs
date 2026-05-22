//! Shared local-context search used by both the heuristic agent path and the
//! ReAct dispatch path. Aggregates workspace metadata, built-in commands,
//! recent notes, history entries, and the local model catalog into a
//! `Vec<GroundingSource>`.
//!
//! Lives under `core/` so the upcoming `core/ai_capability/` layer (REF.4)
//! can reuse it without depending on a handler module.

use std::sync::{Arc, Mutex};

use crate::core::grounding::{source, truncate, visibility_filtered_source};
use crate::core::BuiltinCommandRegistry;
use crate::managers::{
    history_manager::HistoryManager, model_manager::ModelManager, note_manager::NoteManager,
    workspace_manager::WorkspaceManager,
};
use crate::models::agent::{ContextVisibility, GroundingSource};

pub struct LocalContextSearcher {
    pub workspace_manager: Arc<Mutex<WorkspaceManager>>,
    pub note_manager: Arc<Mutex<NoteManager>>,
    pub history_manager: Arc<Mutex<HistoryManager>>,
    pub builtin_registry: Arc<Mutex<BuiltinCommandRegistry>>,
    pub model_manager: Arc<ModelManager>,
}

impl LocalContextSearcher {
    pub fn push_workspace_source(&self, sources: &mut Vec<GroundingSource>) {
        let Ok(workspace) = self.workspace_manager.lock() else {
            return;
        };
        let current = workspace.current();
        let snippet = format!(
            "name={}, mode={}, panel={}, recent_files={}, notes={}",
            current.name,
            current.mode,
            current.panel.as_deref().unwrap_or("none"),
            current.recent_files.len(),
            current.note_ids.len()
        );
        sources.push(source(
            format!("workspace:{}", current.id),
            "workspace",
            current.name.clone(),
            snippet,
            0.92,
            ContextVisibility::PublicContext,
        ));
    }

    pub fn push_command_sources(
        &self,
        query: &str,
        sources: &mut Vec<GroundingSource>,
    ) -> Result<(), String> {
        let registry = self.builtin_registry.lock().map_err(|e| e.to_string())?;
        for meta in registry.list().into_iter().filter(|meta| {
            query.is_empty()
                || meta.name.contains(query)
                || meta.description.to_lowercase().contains(query)
        }) {
            sources.push(source(
                format!("command:{}", meta.name),
                "command",
                format!("/{}", meta.name),
                meta.description.to_string(),
                0.84,
                ContextVisibility::PublicContext,
            ));
        }
        Ok(())
    }

    pub fn push_note_sources(
        &self,
        query: &str,
        sources: &mut Vec<GroundingSource>,
    ) -> Result<(), String> {
        let notes = self.note_manager.lock().map_err(|e| e.to_string())?;
        for note in notes.list() {
            let content = notes.get(&note.name).unwrap_or_default();
            let searchable = format!("{} {}", note.name, content).to_lowercase();
            if !query.is_empty() && !searchable.contains(query) {
                continue;
            }
            let snippet = truncate(&content.replace('\n', " "), 160);
            sources.push(visibility_filtered_source(
                format!("note:{}", note.name),
                "note",
                note.name,
                if snippet.is_empty() {
                    format!("{} bytes", note.size_bytes)
                } else {
                    snippet
                },
                0.78,
            ));
        }
        Ok(())
    }

    pub fn push_history_sources(
        &self,
        query: &str,
        sources: &mut Vec<GroundingSource>,
    ) -> Result<(), String> {
        let history = self.history_manager.lock().map_err(|e| e.to_string())?;
        for entry in history.search(query) {
            sources.push(visibility_filtered_source(
                format!("history:{}", entry.id),
                "history",
                entry.content_type.clone(),
                truncate(&entry.content.replace('\n', " "), 160),
                if entry.pinned { 0.7 } else { 0.62 },
            ));
        }
        Ok(())
    }

    pub fn push_model_sources(&self, query: &str, sources: &mut Vec<GroundingSource>) {
        let hardware = crate::managers::model_manager::HardwareInfo {
            ram_mb: 0,
            vram_mb: 0,
        };
        for model in self
            .model_manager
            .catalog_fast(&hardware)
            .into_iter()
            .filter(|model| query.is_empty() || model.name.to_lowercase().contains(query))
        {
            sources.push(source(
                format!("model:{}", model.name),
                "model",
                model.name,
                model.rating,
                0.68,
                ContextVisibility::PublicContext,
            ));
        }
    }
}