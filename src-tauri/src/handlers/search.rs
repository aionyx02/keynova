use std::collections::HashSet;
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use crate::core::{
    observability, ActionArena, AppEvent, BuiltinCommandRegistry, CommandHandler, CommandResult,
    EventBus,
};
use crate::managers::{
    history_manager::HistoryManager,
    model_manager::{HardwareInfo, ModelManager},
    note_manager::NoteManager,
    search_manager::{SearchBackend, SearchManager},
};
use crate::models::action::{Action, UiSearchItem};
use crate::models::search_result::{ResultKind, SearchResult};

const DEFAULT_FIRST_BATCH_LIMIT: usize = 30;
const DEFAULT_CHUNK_SIZE: usize = 30;
const PROVIDER_TIMEOUT: Duration = Duration::from_millis(120);

#[derive(Clone)]
pub struct SearchHandler {
    manager: Arc<Mutex<SearchManager>>,
    action_arena: Arc<ActionArena>,
    builtin_registry: Arc<Mutex<BuiltinCommandRegistry>>,
    note_manager: Arc<Mutex<NoteManager>>,
    history_manager: Arc<Mutex<HistoryManager>>,
    workspace_manager: Arc<Mutex<crate::managers::workspace_manager::WorkspaceManager>>,
    model_manager: Arc<ModelManager>,
    event_bus: EventBus,
}

pub struct SearchHandlerDeps {
    pub manager: Arc<Mutex<SearchManager>>,
    pub action_arena: Arc<ActionArena>,
    pub builtin_registry: Arc<Mutex<BuiltinCommandRegistry>>,
    pub note_manager: Arc<Mutex<NoteManager>>,
    pub history_manager: Arc<Mutex<HistoryManager>>,
    pub workspace_manager: Arc<Mutex<crate::managers::workspace_manager::WorkspaceManager>>,
    pub model_manager: Arc<ModelManager>,
    pub event_bus: EventBus,
}

impl SearchHandler {
    pub fn new(deps: SearchHandlerDeps) -> Self {
        Self {
            manager: deps.manager,
            action_arena: deps.action_arena,
            builtin_registry: deps.builtin_registry,
            note_manager: deps.note_manager,
            history_manager: deps.history_manager,
            workspace_manager: deps.workspace_manager,
            model_manager: deps.model_manager,
            event_bus: deps.event_bus,
        }
    }
}

impl CommandHandler for SearchHandler {
    fn namespace(&self) -> &'static str {
        "search"
    }

    fn execute(&self, command: &str, payload: Value) -> CommandResult {
        match command {
            "query" => self.execute_query(payload),
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
            "record_selection" => {
                let source = payload
                    .get("source")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown");
                let path = payload.get("path").and_then(Value::as_str).unwrap_or("");
                if !path.is_empty() {
                    self.manager
                        .lock()
                        .map_err(|e| e.to_string())?
                        .record_selection(source, path);
                }
                Ok(json!({ "ok": true }))
            }
            "icon" => Ok(self.icon_payload(&payload)),
            "metadata" => Ok(self.metadata_payload(&payload)),
            _ => Err(format!("search: unknown command '{command}'")),
        }
    }
}

impl SearchHandler {
    fn execute_query(&self, payload: Value) -> CommandResult {
        let query = payload["query"]
            .as_str()
            .ok_or("missing field: query")?
            .to_string();
        let limit = payload["limit"].as_u64().unwrap_or(10) as usize;
        if payload["stream"].as_bool().unwrap_or(false) {
            let request_id = payload["request_id"].as_str().unwrap_or("").to_string();
            let first_batch_limit = payload["first_batch_limit"]
                .as_u64()
                .map(|value| value as usize)
                .unwrap_or(DEFAULT_FIRST_BATCH_LIMIT)
                .min(limit)
                .max(1);
            return self.execute_stream_query(query, limit, first_batch_limit, request_id);
        }
        self.execute_sync_query(query, limit)
    }

    fn execute_sync_query(&self, query: String, limit: usize) -> CommandResult {
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
        self.append_non_file_results(&query, limit, &session, &mut results)?;
        if !self.is_generation_current(generation)? {
            return Ok(json!(Vec::<UiSearchItem>::new()));
        }
        sort_truncate(&mut results, limit);
        observability::log_search_query(
            backend,
            query.chars().count(),
            limit,
            results.len(),
            started.elapsed(),
        );
        serde_json::to_value(results).map_err(|e| e.to_string())
    }

    fn execute_stream_query(
        &self,
        query: String,
        limit: usize,
        first_batch_limit: usize,
        request_id: String,
    ) -> CommandResult {
        let started = Instant::now();
        let session = self.action_arena.start_session();
        let (generation, backend) = {
            let mgr = self.manager.lock().map_err(|e| e.to_string())?;
            (mgr.begin_generation(), mgr.backend)
        };

        let mut first_batch = self.fast_results(&query, first_batch_limit, &session)?;
        sort_truncate(&mut first_batch, first_batch_limit);
        let first_keys = result_keys(&first_batch);
        observability::log_search_query(
            backend.as_str(),
            query.chars().count(),
            first_batch_limit,
            first_batch.len(),
            started.elapsed(),
        );

        if limit > first_batch_limit && !query.trim().is_empty() {
            let worker = self.clone();
            std::thread::spawn(move || {
                worker.run_stream_worker(StreamWorkerRequest {
                    query,
                    limit,
                    request_id,
                    generation,
                    backend,
                    session,
                    seen: first_keys,
                });
            });
        } else {
            self.emit_search_chunk(SearchChunk {
                request_id,
                generation,
                chunk_index: 1,
                items: Vec::new(),
                done: true,
                timed_out_providers: Vec::new(),
            });
        }

        serde_json::to_value(first_batch).map_err(|e| e.to_string())
    }

    fn run_stream_worker(&self, request: StreamWorkerRequest) {
        let StreamWorkerRequest {
            query,
            limit,
            request_id,
            generation,
            backend,
            session,
            mut seen,
        } = request;

        if !self.is_generation_current(generation).unwrap_or(false) {
            return;
        }

        let mut timed_out_providers = Vec::new();
        let mut results = match self.fast_results(&query, limit, &session) {
            Ok(results) => results,
            Err(error) => {
                self.emit_search_error(&request_id, generation, error);
                return;
            }
        };

        match self.file_results_with_timeout(backend, query.clone(), limit) {
            Some(file_results) => match self.base_results_to_ui_items(file_results, &session) {
                Ok(mut file_items) => results.append(&mut file_items),
                Err(error) => {
                    self.emit_search_error(&request_id, generation, error);
                    return;
                }
            },
            None => timed_out_providers.push(backend.as_str().to_string()),
        }

        if !self.is_generation_current(generation).unwrap_or(false) {
            return;
        }

        sort_truncate(&mut results, limit);
        let mut chunk_index = 1usize;
        let mut chunk = Vec::new();
        for item in results {
            let key = search_item_key(&item);
            if !seen.insert(key) {
                continue;
            }
            chunk.push(item);
            if chunk.len() >= DEFAULT_CHUNK_SIZE {
                self.emit_search_chunk(SearchChunk {
                    request_id: request_id.clone(),
                    generation,
                    chunk_index,
                    items: std::mem::take(&mut chunk),
                    done: false,
                    timed_out_providers: timed_out_providers.clone(),
                });
                chunk_index += 1;
            }
        }

        self.emit_search_chunk(SearchChunk {
            request_id,
            generation,
            chunk_index,
            items: chunk,
            done: true,
            timed_out_providers,
        });
    }

    fn fast_results(
        &self,
        query: &str,
        limit: usize,
        session: &crate::core::action_registry::ActionSession,
    ) -> Result<Vec<UiSearchItem>, String> {
        let app_results = {
            let mgr = self.manager.lock().map_err(|e| e.to_string())?;
            mgr.app_results(query, limit)
        };
        let mut results = self.base_results_to_ui_items(app_results, session)?;
        self.append_non_file_results(query, limit, session, &mut results)?;
        Ok(results)
    }

    fn append_non_file_results(
        &self,
        query: &str,
        limit: usize,
        session: &crate::core::action_registry::ActionSession,
        out: &mut Vec<UiSearchItem>,
    ) -> Result<(), String> {
        self.append_command_results(query, limit, session, out)?;
        self.append_note_results(query, limit, session, out)?;
        self.append_history_results(query, limit, session, out)?;
        self.append_model_results(query, limit, session, out)?;
        Ok(())
    }

    fn file_results_with_timeout(
        &self,
        backend: SearchBackend,
        query: String,
        limit: usize,
    ) -> Option<Vec<SearchResult>> {
        let (tx, rx) = mpsc::channel();
        let tantivy_index_dir = self
            .manager
            .lock()
            .ok()
            .map(|manager| manager.tantivy_index_dir());
        std::thread::spawn(move || {
            let _ = tx.send(SearchManager::file_results_for_backend(
                backend,
                &query,
                limit,
                tantivy_index_dir.as_deref(),
            ));
        });
        rx.recv_timeout(PROVIDER_TIMEOUT).ok()
    }

    fn emit_search_chunk(&self, chunk: SearchChunk) {
        let _ = self.event_bus.publish(AppEvent::new(
            "search.results.chunk",
            json!({
                "request_id": chunk.request_id,
                "generation": chunk.generation,
                "chunk_index": chunk.chunk_index,
                "items": chunk.items,
                "done": chunk.done,
                "timed_out_providers": chunk.timed_out_providers,
            }),
        ));
    }

    fn emit_search_error(&self, request_id: &str, generation: u64, error: String) {
        let _ = self.event_bus.publish(AppEvent::new(
            "search.results.error",
            json!({
                "request_id": request_id,
                "generation": generation,
                "error": error,
            }),
        ));
    }

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
                let source = source.to_string();
                let icon_key = icon_key_for_item(&source, &result.path, &result.kind);
                let mut item = UiSearchItem {
                    item_ref: action_ref.clone(),
                    title: result.name.clone(),
                    subtitle: result.path.clone(),
                    source,
                    score: result.score,
                    icon_key: Some(icon_key),
                    primary_action: action_ref,
                    primary_action_label: "Open".into(),
                    secondary_action_count: 1,
                    kind: result.kind,
                    name: result.name,
                    path: result.path,
                };
                self.apply_rank_boost(&mut item);
                Ok(item)
            })
            .collect()
    }

    fn apply_rank_boost(&self, item: &mut UiSearchItem) {
        if let Ok(manager) = self.manager.lock() {
            item.score += manager.rank_boost(&item.source, &item.path);
        }
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
            let mut item = UiSearchItem {
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
            };
            self.apply_rank_boost(&mut item);
            out.push(item);
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
            let mut item = UiSearchItem {
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
            };
            self.apply_rank_boost(&mut item);
            out.push(item);
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
        let workspace_id = self
            .workspace_manager
            .lock()
            .ok()
            .map(|workspace| workspace.current().id);
        let history = self.history_manager.lock().map_err(|e| e.to_string())?;
        for entry in history
            .search_ranked(query, workspace_id)
            .into_iter()
            .take(limit)
        {
            let action = Action::open_panel(
                format!("history:{}", entry.id),
                "Open history",
                "history",
                entry.id.clone(),
            );
            let action_ref = self.action_arena.insert(session, action)?;
            let snippet = entry.content.chars().take(80).collect::<String>();
            let mut score = if entry.pinned { 70 } else { 65 };
            if workspace_id.is_some() && entry.workspace_id == workspace_id {
                score += 10;
            }
            let mut item = UiSearchItem {
                item_ref: action_ref.clone(),
                title: snippet.clone(),
                subtitle: entry.content_type.clone(),
                source: "history".into(),
                score,
                icon_key: Some("history".into()),
                primary_action: action_ref,
                primary_action_label: "Open history".into(),
                secondary_action_count: 0,
                kind: ResultKind::History,
                name: snippet,
                path: format!("history://{}", entry.id),
            };
            self.apply_rank_boost(&mut item);
            out.push(item);
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
            let mut item = UiSearchItem {
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
            };
            self.apply_rank_boost(&mut item);
            out.push(item);
        }
        Ok(())
    }

    fn metadata_payload(&self, payload: &Value) -> Value {
        let path = payload["path"].as_str().unwrap_or("");
        let metadata = std::fs::metadata(path).ok();
        let modified_ms = metadata
            .as_ref()
            .and_then(|meta| meta.modified().ok())
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|duration| duration.as_millis());
        let preview = metadata
            .as_ref()
            .filter(|meta| meta.is_file() && meta.len() <= 64 * 1024)
            .and_then(|_| std::fs::read_to_string(path).ok())
            .map(|text| text.chars().take(240).collect::<String>());
        json!({
            "path": path,
            "exists": metadata.is_some(),
            "is_dir": metadata.as_ref().is_some_and(|meta| meta.is_dir()),
            "size_bytes": metadata.as_ref().map(|meta| meta.len()),
            "modified_ms": modified_ms,
            "preview": preview,
        })
    }

    fn icon_payload(&self, payload: &Value) -> Value {
        let icon_key = payload
            .get("icon_key")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let kind = payload.get("kind").and_then(Value::as_str).unwrap_or("");
        let path = payload.get("path").and_then(Value::as_str).unwrap_or("");
        let label = icon_label(icon_key, kind, path);
        let color = icon_color(icon_key, kind);
        json!({
            "icon_key": icon_key,
            "mime": "image/svg+xml",
            "data_url": svg_data_url(&label, color),
        })
    }
}

struct SearchChunk {
    request_id: String,
    generation: u64,
    chunk_index: usize,
    items: Vec<UiSearchItem>,
    done: bool,
    timed_out_providers: Vec<String>,
}

struct StreamWorkerRequest {
    query: String,
    limit: usize,
    request_id: String,
    generation: u64,
    backend: SearchBackend,
    session: crate::core::action_registry::ActionSession,
    seen: HashSet<String>,
}

fn sort_truncate(results: &mut Vec<UiSearchItem>, limit: usize) {
    results.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.source.cmp(&right.source))
            .then_with(|| left.title.cmp(&right.title))
    });
    results.truncate(limit);
}

fn result_keys(results: &[UiSearchItem]) -> HashSet<String> {
    results.iter().map(search_item_key).collect()
}

fn search_item_key(item: &UiSearchItem) -> String {
    format!("{}:{}", item.source, item.path)
}

fn icon_key_for_item(source: &str, path: &str, kind: &ResultKind) -> String {
    match kind {
        ResultKind::App => format!("app:{}", stable_hash(path)),
        ResultKind::File => {
            let ext = std::path::Path::new(path)
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("file")
                .to_ascii_lowercase();
            format!("file:{ext}")
        }
        ResultKind::Folder => "folder".into(),
        _ => source.to_string(),
    }
}

fn icon_label(icon_key: &str, kind: &str, path: &str) -> String {
    if let Some(ext) = icon_key.strip_prefix("file:") {
        return ext.chars().take(3).collect::<String>().to_uppercase();
    }
    match kind {
        "app" => "APP".into(),
        "folder" => "DIR".into(),
        "command" => "CMD".into(),
        "note" => "MD".into(),
        "history" => "HIS".into(),
        "model" => "AI".into(),
        _ => std::path::Path::new(path)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.chars().take(3).collect::<String>().to_uppercase())
            .unwrap_or_else(|| "KEY".into()),
    }
}

fn icon_color(icon_key: &str, kind: &str) -> &'static str {
    if icon_key.starts_with("file:") {
        return "#0ea5e9";
    }
    match kind {
        "app" => "#8b5cf6",
        "folder" => "#f59e0b",
        "command" => "#10b981",
        "note" => "#14b8a6",
        "history" => "#71717a",
        "model" => "#d946ef",
        _ => "#64748b",
    }
}

fn svg_data_url(label: &str, color: &str) -> String {
    let safe_label = escape_xml(label);
    let svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="32" height="32" viewBox="0 0 32 32"><rect width="32" height="32" rx="7" fill="{color}"/><text x="16" y="20" text-anchor="middle" font-family="Segoe UI,Arial,sans-serif" font-size="10" font-weight="700" fill="#fff">{safe_label}</text></svg>"##
    );
    format!("data:image/svg+xml;utf8,{}", percent_encode(&svg))
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn percent_encode(value: &str) -> String {
    value
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (byte as char).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

fn stable_hash(value: &str) -> u64 {
    let mut h: u64 = 14695981039346656037;
    for b in value.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(1099511628211);
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_keys_group_files_by_extension() {
        assert_eq!(
            icon_key_for_item("file", "C:/tmp/demo.rs", &ResultKind::File),
            "file:rs"
        );
        assert_eq!(
            icon_key_for_item("file", "C:/tmp/folder", &ResultKind::Folder),
            "folder"
        );
    }
}
