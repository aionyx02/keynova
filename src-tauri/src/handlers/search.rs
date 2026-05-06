use std::sync::{Arc, Mutex};
use std::time::Instant;

use serde_json::{json, Value};

use crate::core::{
    observability, ActionArena, BuiltinCommandRegistry, CommandHandler, CommandResult,
};
use crate::managers::{
    history_manager::HistoryManager,
    model_manager::{HardwareInfo, ModelManager},
    note_manager::NoteManager,
    search_manager::SearchManager,
};
use crate::models::action::{Action, UiSearchItem};
use crate::models::search_result::{ResultKind, SearchResult};

/// 統一搜尋 IPC 處理器（App + 檔案）。
pub struct SearchHandler {
    manager: Arc<Mutex<SearchManager>>,
    action_arena: Arc<ActionArena>,
    builtin_registry: Arc<Mutex<BuiltinCommandRegistry>>,
    note_manager: Arc<Mutex<NoteManager>>,
    history_manager: Arc<Mutex<HistoryManager>>,
    model_manager: Arc<ModelManager>,
}

impl SearchHandler {
    pub fn new(
        manager: Arc<Mutex<SearchManager>>,
        action_arena: Arc<ActionArena>,
        builtin_registry: Arc<Mutex<BuiltinCommandRegistry>>,
        note_manager: Arc<Mutex<NoteManager>>,
        history_manager: Arc<Mutex<HistoryManager>>,
        model_manager: Arc<ModelManager>,
    ) -> Self {
        Self {
            manager,
            action_arena,
            builtin_registry,
            note_manager,
            history_manager,
            model_manager,
        }
    }
}

impl CommandHandler for SearchHandler {
    fn namespace(&self) -> &'static str {
        "search"
    }

    fn execute(&self, command: &str, payload: Value) -> CommandResult {
        match command {
            "query" => {
                let query = payload["query"]
                    .as_str()
                    .ok_or("missing field: query")?
                    .to_string();
                let limit = payload["limit"].as_u64().unwrap_or(10) as usize;
                let started = Instant::now();
                let session = self.action_arena.start_session();
                let (generation, backend, base_results) = {
                    let mgr = self.manager.lock().map_err(|e| e.to_string())?;
                    let generation = mgr.begin_generation();
                    let backend = mgr.active_backend_name();
                    let base_results = mgr.search(&query, limit);
                    (generation, backend, base_results)
                };
                let mut results = self.base_results_to_ui_items(base_results, &session)?;
                if !self.is_generation_current(generation)? {
                    return Ok(json!(Vec::<UiSearchItem>::new()));
                }
                self.append_command_results(&query, limit, &session, &mut results)?;
                if !self.is_generation_current(generation)? {
                    return Ok(json!(Vec::<UiSearchItem>::new()));
                }
                self.append_note_results(&query, limit, &session, &mut results)?;
                if !self.is_generation_current(generation)? {
                    return Ok(json!(Vec::<UiSearchItem>::new()));
                }
                self.append_history_results(&query, limit, &session, &mut results)?;
                if !self.is_generation_current(generation)? {
                    return Ok(json!(Vec::<UiSearchItem>::new()));
                }
                self.append_model_results(&query, limit, &session, &mut results)?;
                if !self.is_generation_current(generation)? {
                    return Ok(json!(Vec::<UiSearchItem>::new()));
                }
                results.sort_by(|left, right| {
                    right
                        .score
                        .cmp(&left.score)
                        .then_with(|| left.source.cmp(&right.source))
                        .then_with(|| left.title.cmp(&right.title))
                });
                results.truncate(limit);
                observability::log_search_query(
                    backend,
                    query.chars().count(),
                    limit,
                    results.len(),
                    started.elapsed(),
                );
                Ok(serde_json::to_value(results).map_err(|e| e.to_string())?)
            }
            "cancel" => {
                let generation = self
                    .manager
                    .lock()
                    .map_err(|e| e.to_string())?
                    .cancel_generation();
                Ok(json!({ "ok": true, "generation": generation }))
            }
            "backend" => {
                let mgr = self.manager.lock().map_err(|e| e.to_string())?;
                Ok(serde_json::to_value(mgr.backend_info()).map_err(|e| e.to_string())?)
            }
            "rebuild_index" => {
                let mgr = self.manager.lock().map_err(|e| e.to_string())?;
                Ok(serde_json::to_value(mgr.rebuild_index()).map_err(|e| e.to_string())?)
            }
            _ => Err(format!("search: unknown command '{command}'")),
        }
    }
}

impl SearchHandler {
    fn is_generation_current(&self, generation: u64) -> Result<bool, String> {
        Ok(self
            .manager
            .lock()
            .map_err(|e| e.to_string())?
            .is_current_generation(generation))
    }

    fn base_results_to_ui_items(
        &self,
        results: Vec<SearchResult>,
        session: &crate::core::action_registry::ActionSession,
    ) -> Result<Vec<UiSearchItem>, String> {
        results
            .into_iter()
            .map(|result| {
                let mut action = Action::launch_path(result.path.clone());
                action.secondary_actions.push(Action::open_panel(
                    format!("inspect:{}", result.path),
                    "Inspect",
                    "system",
                    result.path.clone(),
                ));
                let action_ref = self.action_arena.insert(session, action)?;
                let source = match result.kind {
                    ResultKind::App => "app",
                    ResultKind::File => "file",
                    ResultKind::Folder => "file",
                    ResultKind::Command => "command",
                    ResultKind::Note => "note",
                    ResultKind::History => "history",
                    ResultKind::Model => "model",
                };
                Ok(UiSearchItem {
                    item_ref: action_ref.clone(),
                    title: result.name.clone(),
                    subtitle: result.path.clone(),
                    source: source.to_string(),
                    score: result.score,
                    icon_key: Some(source.to_string()),
                    primary_action: action_ref,
                    primary_action_label: "Open".into(),
                    secondary_action_count: 1,
                    kind: result.kind,
                    name: result.name,
                    path: result.path,
                })
            })
            .collect()
    }

    fn append_command_results(
        &self,
        query: &str,
        limit: usize,
        session: &crate::core::action_registry::ActionSession,
        out: &mut Vec<UiSearchItem>,
    ) -> Result<(), String> {
        let q = query.to_lowercase();
        let registry = self.builtin_registry.lock().map_err(|e| e.to_string())?;
        for meta in registry
            .list()
            .into_iter()
            .filter(|meta| meta.name.contains(&q) || meta.description.to_lowercase().contains(&q))
            .take(limit)
        {
            let action = Action::command_route(
                format!("cmd:{}", meta.name),
                format!("/{}", meta.name),
                "cmd.run",
                json!({ "name": meta.name, "args": "" }),
            );
            let action_ref = self.action_arena.insert(session, action)?;
            out.push(UiSearchItem {
                item_ref: action_ref.clone(),
                title: format!("/{}", meta.name),
                subtitle: meta.description.to_string(),
                source: "command".into(),
                score: 85,
                icon_key: Some("command".into()),
                primary_action: action_ref,
                primary_action_label: "Run".into(),
                secondary_action_count: 0,
                kind: ResultKind::Command,
                name: meta.name.to_string(),
                path: format!("command://{}", meta.name),
            });
        }
        Ok(())
    }

    fn append_note_results(
        &self,
        query: &str,
        limit: usize,
        session: &crate::core::action_registry::ActionSession,
        out: &mut Vec<UiSearchItem>,
    ) -> Result<(), String> {
        let q = query.to_lowercase();
        let notes = self.note_manager.lock().map_err(|e| e.to_string())?.list();
        for note in notes
            .into_iter()
            .filter(|note| note.name.to_lowercase().contains(&q))
            .take(limit)
        {
            let action = Action::open_panel(
                format!("note:{}", note.name),
                "Open note",
                "note",
                note.name.clone(),
            );
            let action_ref = self.action_arena.insert(session, action)?;
            out.push(UiSearchItem {
                item_ref: action_ref.clone(),
                title: note.name.clone(),
                subtitle: format!("{} bytes", note.size_bytes),
                source: "note".into(),
                score: 75,
                icon_key: Some("note".into()),
                primary_action: action_ref,
                primary_action_label: "Open note".into(),
                secondary_action_count: 0,
                kind: ResultKind::Note,
                name: note.name.clone(),
                path: format!("note://{}", note.name),
            });
        }
        Ok(())
    }

    fn append_history_results(
        &self,
        query: &str,
        limit: usize,
        session: &crate::core::action_registry::ActionSession,
        out: &mut Vec<UiSearchItem>,
    ) -> Result<(), String> {
        let history = self.history_manager.lock().map_err(|e| e.to_string())?;
        for entry in history.search(query).into_iter().take(limit) {
            let action = Action::open_panel(
                format!("history:{}", entry.id),
                "Open history",
                "history",
                entry.id.clone(),
            );
            let action_ref = self.action_arena.insert(session, action)?;
            let snippet = entry.content.chars().take(80).collect::<String>();
            out.push(UiSearchItem {
                item_ref: action_ref.clone(),
                title: snippet.clone(),
                subtitle: entry.content_type.clone(),
                source: "history".into(),
                score: if entry.pinned { 70 } else { 65 },
                icon_key: Some("history".into()),
                primary_action: action_ref,
                primary_action_label: "Open history".into(),
                secondary_action_count: 0,
                kind: ResultKind::History,
                name: snippet,
                path: format!("history://{}", entry.id),
            });
        }
        Ok(())
    }

    fn append_model_results(
        &self,
        query: &str,
        limit: usize,
        session: &crate::core::action_registry::ActionSession,
        out: &mut Vec<UiSearchItem>,
    ) -> Result<(), String> {
        let q = query.to_lowercase();
        let hardware = HardwareInfo {
            ram_mb: 0,
            vram_mb: 0,
        };
        for model in self
            .model_manager
            .catalog_fast(&hardware)
            .into_iter()
            .filter(|model| model.name.to_lowercase().contains(&q))
            .take(limit)
        {
            let action = Action::open_panel(
                format!("model:{}", model.name),
                "Open model",
                "model_list",
                model.name.clone(),
            );
            let action_ref = self.action_arena.insert(session, action)?;
            out.push(UiSearchItem {
                item_ref: action_ref.clone(),
                title: model.name.clone(),
                subtitle: model.rating.clone(),
                source: "model".into(),
                score: 60,
                icon_key: Some("model".into()),
                primary_action: action_ref,
                primary_action_label: "Manage model".into(),
                secondary_action_count: 0,
                kind: ResultKind::Model,
                name: model.name.clone(),
                path: format!("model://{}", model.name),
            });
        }
        Ok(())
    }
}
