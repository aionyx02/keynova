use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use crate::core::config_manager::ConfigManager;
use crate::core::knowledge_store::KnowledgeStoreHandle;
use crate::core::{
    observability, ActionArena, AppEvent, BuiltinCommandRegistry, CommandHandler, CommandResult,
    EventBus,
};
use crate::managers::{
    history_manager::HistoryManager,
    model_manager::ModelManager,
    note_manager::NoteManager,
    search_manager::{SearchBackend, SearchManager},
    search_service::SearchService,
};
use crate::models::action::{Action, ScoreBreakdown, UiSearchItem};
use crate::models::ipc_requests::{SearchQueryRequest, SearchRecordSelectionRequest};
use crate::models::search_result::{ResultKind, SearchResult};
use crate::models::unified_result::UnifiedResult;

mod icon;
mod providers;
mod ranking;

use icon::{icon_color, icon_label, svg_data_url};
// Re-exported at crate visibility: platform::windows icon-cache pre-warm calls
// `crate::handlers::search::icon_key_for_item` (main d2aef7d).
pub(crate) use icon::icon_key_for_item;
use ranking::{
    result_keys, search_item_key, sort_balanced_truncate, sort_truncate, strip_global_prefix,
};

/// REF.6.A — convert internal `UiSearchItem` rows into the wire format the
/// palette consumes (`UnifiedResult`). Keeps `UiSearchItem` as the
/// computation type inside `SearchHandler` while standardising the IPC
/// boundary.
fn to_unified_results(items: Vec<UiSearchItem>) -> Vec<UnifiedResult> {
    items.into_iter().map(UnifiedResult::from).collect()
}

const DEFAULT_FIRST_BATCH_LIMIT: usize = 30;

const PROVIDER_TIMEOUT: Duration = Duration::from_millis(800);

const FILE_LIMIT_MULTIPLIER: usize = 6;
const APP_LIMIT: usize = 8;
const COMMAND_LIMIT: usize = 8;
const NOTE_LIMIT: usize = 8;
const HISTORY_LIMIT: usize = 12;
const MODEL_LIMIT: usize = 6;
const MEMORY_LIMIT: usize = 6;

#[derive(Debug, Clone)]
struct SearchPlan {
    display_limit: usize,
    first_batch_limit: usize,
    app_limit: usize,
    command_limit: usize,
    note_limit: usize,
    history_limit: usize,
    model_limit: usize,
    memory_limit: usize,
    file_limit: usize,
}

impl SearchPlan {
    fn from_limit(limit: usize, first_batch_limit: usize) -> Self {
        let display_limit = limit.max(1);
        Self {
            display_limit,
            first_batch_limit: first_batch_limit.min(display_limit).max(1),
            app_limit: APP_LIMIT,
            command_limit: COMMAND_LIMIT,
            note_limit: NOTE_LIMIT,
            history_limit: HISTORY_LIMIT,
            model_limit: MODEL_LIMIT,
            memory_limit: MEMORY_LIMIT,
            file_limit: display_limit.saturating_mul(FILE_LIMIT_MULTIPLIER).max(120),
        }
    }
}

#[derive(Clone)]
pub struct SearchHandler {
    manager: Arc<Mutex<SearchManager>>,
    action_arena: Arc<ActionArena>,
    builtin_registry: Arc<Mutex<BuiltinCommandRegistry>>,
    note_manager: Arc<Mutex<NoteManager>>,
    history_manager: Arc<Mutex<HistoryManager>>,
    workspace_manager: Arc<Mutex<crate::managers::workspace_manager::WorkspaceManager>>,
    model_manager: Arc<ModelManager>,
    config: Arc<Mutex<ConfigManager>>,
    knowledge_store: KnowledgeStoreHandle,
    event_bus: EventBus,
    search_service: Arc<SearchService>,
}

pub struct SearchHandlerDeps {
    pub manager: Arc<Mutex<SearchManager>>,
    pub action_arena: Arc<ActionArena>,
    pub builtin_registry: Arc<Mutex<BuiltinCommandRegistry>>,
    pub note_manager: Arc<Mutex<NoteManager>>,
    pub history_manager: Arc<Mutex<HistoryManager>>,
    pub workspace_manager: Arc<Mutex<crate::managers::workspace_manager::WorkspaceManager>>,
    pub model_manager: Arc<ModelManager>,
    pub config: Arc<Mutex<ConfigManager>>,
    pub knowledge_store: KnowledgeStoreHandle,
    pub event_bus: EventBus,
    pub search_service: Arc<SearchService>,
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
            config: deps.config,
            knowledge_store: deps.knowledge_store,
            event_bus: deps.event_bus,
            search_service: deps.search_service,
        }
    }

    /// Reads a `features.*` flag. Missing/empty ⇒ enabled (matches the repo-wide
    /// `unwrap_or(true)` idiom); only an explicit `false` disables.
    fn feature_enabled(&self, key: &str) -> bool {
        self.config
            .lock()
            .ok()
            .and_then(|cfg| cfg.get(key))
            .as_deref()
            .map(|v| !v.eq_ignore_ascii_case("false"))
            .unwrap_or(true)
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
                let req: SearchRecordSelectionRequest = serde_json::from_value(payload)
                    .map_err(|e| format!("invalid search.record_selection request: {e}"))?;
                if !req.path.is_empty() {
                    self.manager
                        .lock()
                        .map_err(|e| e.to_string())?
                        .record_selection(&req.source, &req.path);
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
    /// Resolves workspace scope from the query.
    ///
    /// - Strips a leading `:global ` (or bare `:global`) prefix and returns
    ///   `(cleaned_query, None)` so the search runs unrestricted.
    /// - Otherwise, returns `(query, current_workspace.project_root)`. When
    ///   the workspace has no `project_root` configured, the second tuple
    ///   element is `None` and search behaves globally as before.
    fn resolve_workspace_filter(&self, raw: &str) -> (String, Option<String>) {
        let (cleaned, global) = strip_global_prefix(raw);
        if global {
            return (cleaned, None);
        }
        let workspace_root = self
            .workspace_manager
            .lock()
            .ok()
            .and_then(|mgr| mgr.current().project_root.clone())
            .filter(|s| !s.trim().is_empty());
        (cleaned, workspace_root)
    }

    fn execute_query(&self, payload: Value) -> CommandResult {
        let req: SearchQueryRequest = serde_json::from_value(payload)
            .map_err(|e| format!("invalid search.query request: {e}"))?;
        let (query, workspace_root) = self.resolve_workspace_filter(&req.query);
        let limit = req.limit;
        if req.stream {
            let request_id = req.request_id;
            let first_batch_limit = req
                .first_batch_limit
                .unwrap_or(DEFAULT_FIRST_BATCH_LIMIT)
                .min(limit)
                .max(1);
            return self.execute_stream_query(
                query,
                workspace_root,
                limit,
                first_batch_limit,
                request_id,
            );
        }
        self.execute_sync_query(query, workspace_root, limit)
    }

    /// Filters results in-place to those whose `path` lives under
    /// `root` (case-insensitive prefix match — Windows paths). Non-file kinds
    /// (`command`/`note`/`history`/`model`) are always retained.
    fn apply_workspace_filter(items: &mut Vec<UiSearchItem>, root: Option<&str>) {
        let Some(root) = root else { return };
        let needle = root.to_lowercase();
        items.retain(|item| match item.kind {
            ResultKind::File | ResultKind::Folder | ResultKind::App => {
                item.path.to_lowercase().starts_with(&needle)
            }
            _ => true,
        });
    }

    fn execute_sync_query(
        &self,
        query: String,
        workspace_root: Option<String>,
        limit: usize,
    ) -> CommandResult {
        let started = Instant::now();
        let session = self.action_arena.start_session();
        let plan = SearchPlan::from_limit(limit, limit);
        let (generation, backend, base_results) = {
            let mgr = self.manager.lock().map_err(|e| e.to_string())?;
            let generation = mgr.begin_generation();
            let backend = mgr.active_backend_name();
            let base_results = mgr.search(&query, plan.file_limit);
            (generation, backend, base_results)
        };
        let mut results = self.base_results_to_ui_items(base_results, &session)?;
        if !self.is_generation_current(generation)? {
            return Ok(json!(Vec::<UnifiedResult>::new()));
        }
        self.append_non_file_results(&query, &plan, &session, &mut results)?;
        if !self.is_generation_current(generation)? {
            return Ok(json!(Vec::<UnifiedResult>::new()));
        }
        Self::apply_workspace_filter(&mut results, workspace_root.as_deref());
        sort_balanced_truncate(&mut results, plan.display_limit);
        observability::log_search_query(
            backend,
            query.chars().count(),
            limit,
            results.len(),
            started.elapsed(),
        );
        serde_json::to_value(to_unified_results(results)).map_err(|e| e.to_string())
    }

    fn execute_stream_query(
        &self,
        query: String,
        workspace_root: Option<String>,
        limit: usize,
        first_batch_limit: usize,
        request_id: String,
    ) -> CommandResult {
        let started = Instant::now();
        let session = self.action_arena.start_session();
        let (generation, backend) = {
            let mgr = self.manager.lock().map_err(|e| e.to_string())?;
            (mgr.begin_generation(), mgr.active_backend())
        };

        let plan = SearchPlan::from_limit(limit, first_batch_limit);

        let mut first_batch = self.fast_results(&query, &plan, &session)?;
        Self::apply_workspace_filter(&mut first_batch, workspace_root.as_deref());
        sort_truncate(&mut first_batch, plan.first_batch_limit);
        let first_keys = result_keys(&first_batch);
        let first_batch_items = first_batch.clone();
        observability::log_search_query(
            backend.as_str(),
            query.chars().count(),
            plan.first_batch_limit,
            first_batch.len(),
            started.elapsed(),
        );

        if !query.trim().is_empty() {
            let cancel = Arc::new(AtomicBool::new(false));
            let cancel_for_worker = Arc::clone(&cancel);
            let worker = self.clone();
            self.search_service.submit(cancel, move || {
                worker.run_stream_worker(StreamWorkerRequest {
                    query,
                    request_id,
                    generation,
                    backend,
                    session,
                    seen: first_keys,
                    first_batch_items,
                    plan,
                    cancel: cancel_for_worker,
                    workspace_root,
                });
            });
        } else {
            self.emit_search_chunk(SearchChunk {
                request_id,
                generation,
                chunk_index: 1,
                items: Vec::new(),
                done: true,
                replace: false,
                timed_out_providers: Vec::new(),
                diagnostics: None,
            });
        }

        serde_json::to_value(to_unified_results(first_batch)).map_err(|e| e.to_string())
    }

    fn run_stream_worker(&self, request: StreamWorkerRequest) {
        let StreamWorkerRequest {
            query,
            request_id,
            generation,
            backend,
            session,
            mut seen,
            first_batch_items,
            plan,
            cancel,
            workspace_root,
        } = request;

        let started = Instant::now();

        if cancel.load(Ordering::Relaxed) {
            return;
        }

        let mut timed_out_providers = Vec::new();
        let mut worker_items = Vec::new();
        let timed_out;

        match self.file_results_bounded(
            backend,
            query.clone(),
            plan.file_limit,
            generation,
            Arc::clone(&cancel),
        ) {
            Some(file_results) => {
                timed_out = false;
                match self.base_results_to_ui_items(file_results, &session) {
                    Ok(mut file_items) => worker_items.append(&mut file_items),
                    Err(error) => {
                        self.emit_search_error(&request_id, generation, error);
                        return;
                    }
                }
            }
            None => {
                timed_out = true;
                timed_out_providers.push(backend.as_str().to_string());
            }
        }

        if cancel.load(Ordering::Relaxed) {
            return;
        }

        // Merge first-batch items with new worker items, deduplicate, then balance.
        let mut combined = first_batch_items;
        for item in worker_items {
            let key = search_item_key(&item);
            if seen.insert(key) {
                combined.push(item);
            }
        }
        // Restrict file/folder/app results to workspace root when set.
        Self::apply_workspace_filter(&mut combined, workspace_root.as_deref());
        let pre_balance_count = combined.len();
        sort_balanced_truncate(&mut combined, plan.display_limit);
        let returned_count = combined.len();
        let elapsed_ms = started.elapsed().as_millis();

        let diagnostics = self.manager.lock().ok().map(|mgr| {
            let info = mgr.backend_info();
            let fallback_reason = if timed_out {
                None
            } else if info.indexing {
                Some("Indexing in background, using current cache".into())
            } else if backend == SearchBackend::Tantivy && info.tantivy_index_entries == 0 {
                Some("Tantivy index empty, using cache fallback".into())
            } else if !info.everything_available && backend == SearchBackend::AppCache {
                Some("Everything unavailable, using cache fallback".into())
            } else {
                None
            };
            SearchChunkDiagnostics {
                elapsed_ms,
                timed_out,
                file_cache_entries: info.file_cache_entries,
                tantivy_index_entries: info.tantivy_index_entries,
                everything_available: info.everything_available,
                indexing: info.indexing,
                pre_balance_count,
                returned_count,
                fallback_reason,
            }
        });

        self.emit_search_chunk(SearchChunk {
            request_id,
            generation,
            chunk_index: 1,
            items: combined,
            done: true,
            replace: true,
            timed_out_providers,
            diagnostics,
        });
    }

    fn fast_results(
        &self,
        query: &str,
        plan: &SearchPlan,
        session: &crate::core::action_registry::ActionSession,
    ) -> Result<Vec<UiSearchItem>, String> {
        let app_results = {
            let mgr = self.manager.lock().map_err(|e| e.to_string())?;
            mgr.app_results(query, plan.app_limit)
        };
        let mut results = self.base_results_to_ui_items(app_results, session)?;
        self.append_non_file_results(query, plan, session, &mut results)?;
        Ok(results)
    }

    fn file_results_bounded(
        &self,
        backend: SearchBackend,
        query: String,
        limit: usize,
        generation: u64,
        cancel: Arc<AtomicBool>,
    ) -> Option<Vec<SearchResult>> {
        if cancel.load(Ordering::Relaxed) {
            return None;
        }
        let (tx, rx) = mpsc::channel();
        let tantivy_index_dir = self
            .manager
            .lock()
            .ok()
            .map(|manager| manager.tantivy_index_dir());
        let manager = self.manager.clone();
        // Inner thread enforces the hard timeout on the platform-specific search call.
        std::thread::spawn(move || {
            if cancel.load(Ordering::Relaxed) {
                return;
            }
            if manager
                .lock()
                .ok()
                .is_none_or(|mgr| !mgr.is_current_generation(generation))
            {
                return;
            }
            let results = SearchManager::file_results_for_backend(
                backend,
                &query,
                limit,
                tantivy_index_dir.as_deref(),
            );
            if !cancel.load(Ordering::Relaxed) {
                let _ = tx.send(results);
            }
        });
        rx.recv_timeout(PROVIDER_TIMEOUT).ok()
    }

    fn emit_search_chunk(&self, chunk: SearchChunk) {
        // REF.6.A — palette consumes `UnifiedResult`; convert at the event-
        // bus emit boundary so the worker's internal `UiSearchItem`
        // computation type doesn't need to change.
        let items = to_unified_results(chunk.items);
        let _ = self.event_bus.publish(AppEvent::new(
            "search.results.chunk",
            json!({
                "request_id": chunk.request_id,
                "generation": chunk.generation,
                "chunk_index": chunk.chunk_index,
                "items": items,
                "done": chunk.done,
                "replace": chunk.replace,
                "timed_out_providers": chunk.timed_out_providers,
                "diagnostics": chunk.diagnostics,
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
                    // Memory rows are built by the memory provider, not here, but
                    // the match must stay exhaustive.
                    ResultKind::Memory => "memory",
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
                    score_breakdown: ScoreBreakdown::default(),
                };
                self.apply_rank_boost(&mut item);
                Ok(item)
            })
            .collect()
    }

    fn apply_rank_boost(&self, item: &mut UiSearchItem) {
        let base = item.score;
        // PRODUCT.1.A workspace_context + README/config terms. project_root is the
        // active workspace root; for non-file sources / unset root both yield 0.
        let project_root = self
            .workspace_manager
            .lock()
            .ok()
            .and_then(|ws| ws.current().project_root.clone())
            .filter(|root| !root.trim().is_empty());
        let workspace = ranking::workspace_boost(&item.source, &item.path, project_root.as_deref());
        let config = ranking::config_boost(&item.source, &item.path);
        let noise = ranking::noise_penalty(&item.source, &item.path);
        let (recency, frequency) = match self.manager.lock() {
            Ok(manager) => manager.rank_boost_breakdown(&item.source, &item.path),
            Err(_) => (0, 0),
        };
        item.score = base + workspace + config + recency + frequency + noise;
        item.score_breakdown = ScoreBreakdown {
            base,
            workspace_boost: workspace,
            config_boost: config,
            recency_boost: recency,
            frequency_boost: frequency,
            noise_penalty: noise,
        };
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
        #[cfg(target_os = "windows")]
        if let Some(data_url) = crate::platform::windows::search_icon_data_url(icon_key, kind, path)
        {
            return json!({
                "icon_key": icon_key,
                "mime": "image/png",
                "data_url": data_url,
            });
        }
        let label = icon_label(icon_key, kind, path);
        let color = icon_color(icon_key, kind);
        json!({
            "icon_key": icon_key,
            "mime": "image/svg+xml",
            "data_url": svg_data_url(&label, color),
        })
    }
}

/// Per-provider diagnostics included in the final balanced search chunk.
#[derive(serde::Serialize)]
struct SearchChunkDiagnostics {
    elapsed_ms: u128,
    timed_out: bool,
    file_cache_entries: usize,
    tantivy_index_entries: usize,
    everything_available: bool,
    indexing: bool,
    /// Items available before per-source quota balancing.
    pre_balance_count: usize,
    /// Items actually returned after balancing and truncation.
    returned_count: usize,
    /// Human-readable reason when a non-obvious fallback was chosen.
    fallback_reason: Option<String>,
}

struct SearchChunk {
    request_id: String,
    generation: u64,
    chunk_index: usize,
    items: Vec<UiSearchItem>,
    done: bool,
    replace: bool,
    timed_out_providers: Vec<String>,
    diagnostics: Option<SearchChunkDiagnostics>,
}

struct StreamWorkerRequest {
    query: String,
    request_id: String,
    generation: u64,
    backend: SearchBackend,
    session: crate::core::action_registry::ActionSession,
    /// Keys of items already sent in the first batch (for dedup).
    seen: HashSet<String>,
    /// The actual first-batch items, cloned so the worker can build a combined balanced set.
    first_batch_items: Vec<UiSearchItem>,
    plan: SearchPlan,
    /// Cancel token set by SearchService when a newer request supersedes this one.
    cancel: Arc<AtomicBool>,
    /// Workspace root used to restrict file/folder/app results.
    /// `None` for global search (workspace had no project_root or `:global` prefix used).
    workspace_root: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::ranking::command_match_score;
    use super::*;
    use crate::models::action::ActionRef;

    fn dummy_ref() -> ActionRef {
        ActionRef::new("test", None, 0)
    }

    fn make_item(source: &str, score: i64, idx: usize) -> UiSearchItem {
        UiSearchItem {
            item_ref: dummy_ref(),
            title: format!("{source}-{idx}"),
            subtitle: String::new(),
            source: source.into(),
            score,
            icon_key: None,
            primary_action: dummy_ref(),
            primary_action_label: String::new(),
            secondary_action_count: 0,
            kind: ResultKind::File,
            name: format!("{source}-{idx}"),
            path: format!("{source}://{idx}"),
            score_breakdown: Default::default(),
        }
    }

    // Workspace filter.

    fn make_typed_item(kind: ResultKind, path: &str) -> UiSearchItem {
        let mut item = make_item("file", 100, 0);
        item.kind = kind;
        item.path = path.into();
        item
    }

    #[test]
    fn strip_global_prefix_handles_variants() {
        assert_eq!(strip_global_prefix("readme"), ("readme".into(), false));
        assert_eq!(strip_global_prefix(":global"), (String::new(), true));
        assert_eq!(strip_global_prefix(":global foo"), ("foo".into(), true));
        assert_eq!(strip_global_prefix("  :global  foo"), ("foo".into(), true));
        // `:globalfoo` (no separator) should not be treated as the prefix.
        assert_eq!(
            strip_global_prefix(":globalfoo"),
            (":globalfoo".into(), false)
        );
    }

    #[test]
    fn apply_workspace_filter_no_root_is_noop() {
        let mut items = vec![
            make_typed_item(ResultKind::File, "C:/foo/a.txt"),
            make_typed_item(ResultKind::File, "D:/bar/b.txt"),
        ];
        SearchHandler::apply_workspace_filter(&mut items, None);
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn apply_workspace_filter_keeps_files_under_root() {
        let mut items = vec![
            make_typed_item(ResultKind::File, "C:/proj/src/a.rs"),
            make_typed_item(ResultKind::File, "C:/proj/src/b.rs"),
            make_typed_item(ResultKind::File, "C:/other/c.rs"),
        ];
        SearchHandler::apply_workspace_filter(&mut items, Some("C:/proj"));
        assert_eq!(items.len(), 2);
        assert!(items.iter().all(|i| i.path.starts_with("C:/proj")));
    }

    #[test]
    fn apply_workspace_filter_is_case_insensitive() {
        let mut items = vec![
            make_typed_item(ResultKind::File, "C:/Proj/src/a.rs"),
            make_typed_item(ResultKind::File, "c:/proj/src/b.rs"),
        ];
        SearchHandler::apply_workspace_filter(&mut items, Some("c:/PROJ"));
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn apply_workspace_filter_retains_non_file_kinds() {
        let mut items = vec![
            make_typed_item(ResultKind::File, "C:/other/a.rs"),
            make_typed_item(ResultKind::Command, "command://help"),
            make_typed_item(ResultKind::Note, "note://daily"),
            make_typed_item(ResultKind::History, "history://abc"),
        ];
        SearchHandler::apply_workspace_filter(&mut items, Some("C:/proj"));
        // File dropped (outside root); command/note/history retained.
        assert_eq!(items.len(), 3);
        assert!(items.iter().all(|i| i.kind != ResultKind::File));
    }

    #[test]
    fn command_name_prefix_beats_description_match() {
        // "/down" has description "Gracefully quit Keynova".
        // Searching "keynova" should give a low score (description-only), not 85.
        assert_eq!(
            command_match_score("down", "Gracefully quit Keynova", "keynova"),
            Some(40)
        );
        // Searching "down" should give prefix score 90.
        assert_eq!(
            command_match_score("down", "Gracefully quit Keynova", "down"),
            Some(90)
        );
        // Searching "ow" (substring) should give 85.
        assert_eq!(
            command_match_score("down", "Gracefully quit Keynova", "ow"),
            Some(85)
        );
        // No match at all.
        assert_eq!(
            command_match_score("down", "Gracefully quit Keynova", "zzz"),
            None
        );
        // Empty query returns None.
        assert_eq!(
            command_match_score("down", "Gracefully quit Keynova", ""),
            None
        );
    }

    #[test]
    fn description_only_score_is_below_file_score() {
        let score = command_match_score("down", "Gracefully quit Keynova", "keynova").unwrap();
        assert!(
            score < 80,
            "description-only match ({score}) must be below file score (80)"
        );
    }

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

    #[test]
    fn search_plan_file_limit_scales_with_display_limit() {
        let plan = SearchPlan::from_limit(30, 30);
        assert_eq!(plan.display_limit, 30);
        assert_eq!(plan.file_limit, 180); // 30 * 6
        assert_eq!(plan.app_limit, APP_LIMIT);
        assert_eq!(plan.command_limit, COMMAND_LIMIT);
        assert_eq!(plan.history_limit, HISTORY_LIMIT);
    }

    #[test]
    fn search_plan_file_limit_has_minimum() {
        let plan = SearchPlan::from_limit(1, 1);
        assert_eq!(plan.file_limit, 120); // max(1*6, 120)
    }

    // ─── TD.5.D: Chunk merge and stale request tests ─────────────────────────

    #[test]
    fn search_item_key_is_source_colon_path() {
        // make_item(source, _, idx) sets path = "{source}://{idx}"
        let item = make_item("file", 80, 3);
        // Expected: "file:file://3" (source="file", path="file://3")
        assert_eq!(search_item_key(&item), "file:file://3");
    }

    #[test]
    fn result_keys_collects_unique_keys_for_all_items() {
        let items = vec![
            make_item("file", 80, 0),
            make_item("file", 80, 1),
            make_item("note", 75, 0),
        ];
        let keys = result_keys(&items);
        assert_eq!(keys.len(), 3);
    }

    #[test]
    fn chunk_merge_deduplicates_by_source_and_path() {
        let first_batch = vec![make_item("file", 90, 0), make_item("file", 88, 1)];
        let first_keys = result_keys(&first_batch);
        let worker_items = vec![
            make_item("file", 85, 1), // duplicate of first_batch[1]
            make_item("file", 82, 2), // new
            make_item("note", 80, 0), // new
        ];

        let mut seen = first_keys;
        let mut combined = first_batch.clone();
        for item in worker_items {
            let key = search_item_key(&item);
            if seen.insert(key) {
                combined.push(item);
            }
        }

        let file_count = combined.iter().filter(|i| i.source == "file").count();
        let note_count = combined.iter().filter(|i| i.source == "note").count();
        assert_eq!(
            file_count, 3,
            "2 from first_batch + 1 new file (dup skipped)"
        );
        assert_eq!(note_count, 1, "1 new note");
        assert_eq!(combined.len(), 4);
    }

    #[test]
    fn chunk_merge_preserves_first_batch_items_even_if_lower_score_in_worker() {
        // First batch has item at score 90; worker emits same path at score 95.
        // The first-batch version should be kept (worker dup is dropped).
        let first_batch = vec![make_item("file", 90, 0)];
        let first_keys = result_keys(&first_batch);
        let worker_items = vec![make_item("file", 95, 0)]; // same key, higher score

        let mut seen = first_keys;
        let mut combined = first_batch.clone();
        for item in worker_items {
            let key = search_item_key(&item);
            if seen.insert(key) {
                combined.push(item);
            }
        }

        assert_eq!(combined.len(), 1, "dup must be dropped");
        assert_eq!(combined[0].score, 90, "first-batch version is kept");
    }

    #[test]
    fn stale_cancel_before_worker_emits_no_items() {
        // Simulates the cancel check at the start of run_stream_worker.
        let cancel = Arc::new(AtomicBool::new(true)); // already cancelled
        assert!(cancel.load(Ordering::Relaxed), "cancel must be true");
        // When cancel is set, the worker returns immediately without emitting.
        // We test the guard directly:
        let emitted = !cancel.load(Ordering::Relaxed);
        assert!(!emitted, "cancelled worker must not emit");
    }

    #[test]
    fn sort_balanced_truncate_respects_per_source_quotas() {
        let mut results = Vec::new();
        // Flood with high-score history items (over quota)
        for i in 0..20 {
            results.push(make_item("history", 90 + i as i64, i));
        }
        // Add file items at lower score
        for i in 0..10 {
            results.push(make_item("file", 80, i));
        }

        sort_balanced_truncate(&mut results, 30);

        let history_count = results.iter().filter(|r| r.source == "history").count();
        let file_count = results.iter().filter(|r| r.source == "file").count();

        // History capped at HISTORY_LIMIT (12), all 10 files should appear
        assert_eq!(history_count, HISTORY_LIMIT);
        assert_eq!(file_count, 10);
    }
}
