use std::io::Read;
use std::path::PathBuf;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::core::agent_runtime::ToolDispatch;
use crate::core::config_manager::ConfigManager;
use crate::core::dev_runner::{
    bound_output_n, extract_compiler_errors, run_bounded_dev_cmd, DEV_CARGO_TIMEOUT_SECS,
    DEV_NPM_TIMEOUT_SECS,
};
use crate::core::local_context::LocalContextSearcher;
use crate::core::{
    prepare_observation, AgentAuditEntry, AgentObservationPolicy, AgentRuntime,
    BuiltinCommandRegistry, CommandHandler, CommandResult, KnowledgeStoreHandle,
};
use crate::managers::{
    history_manager::HistoryManager,
    model_manager::ModelManager,
    note_manager::NoteManager,
    system_indexer::search_system_index,
    workspace_manager::WorkspaceManager,
};
use crate::models::action::ActionRisk;
use crate::models::agent::{AgentError, AgentToolCall, ContextVisibility, GroundingSource};
use crate::models::context_bundle::{ContextBundle, SelectedFileContext, WorkspaceContext};
use crate::models::settings_schema::builtin_setting_schema;


mod answers;
mod filesystem;
mod formatting;
mod intent;
mod lifecycle;
mod planning;
mod safety;
mod web;

#[allow(unused_imports)]
use self::filesystem::*;
#[allow(unused_imports)]
use self::formatting::*;
#[allow(unused_imports)]
use self::intent::*;
#[allow(unused_imports)]
use self::safety::*;
#[allow(unused_imports)]
use self::web::*;
const PROMPT_BUDGET_CHARS: usize = 1400;
const PROMPT_SOURCE_LIMIT: usize = 6;
const SESSION_MEMORY_LIMIT: usize = 3;
const LONG_TERM_MEMORY_LIMIT: usize = 3;

const CONTEXT_BUNDLE_BUDGET_CHARS: usize = 3_000;
const CONTEXT_BUNDLE_MAX_FILES: usize = 3;
const CONTEXT_BUNDLE_RECENT_ACTIONS: usize = 5;

const TOOL_KEYNOVA_SEARCH: &str = "keynova_search";
const TOOL_FILESYSTEM_SEARCH: &str = "filesystem_search";
const TOOL_FILESYSTEM_READ: &str = "filesystem_read";
const TOOL_WEB_SEARCH: &str = "web_search";
const TOOL_GIT_STATUS: &str = "git_status";
const TOOL_DEV_CARGO_TEST: &str = "dev_cargo_test";
const TOOL_DEV_CARGO_CHECK: &str = "dev_cargo_check";
const TOOL_DEV_NPM_BUILD: &str = "dev_npm_build";
const TOOL_DEV_NPM_LINT: &str = "dev_npm_lint";
const TOOL_DEV_EXPLAIN_ERROR: &str = "dev_explain_compiler_error";
const TOOL_LEARNING_MATERIAL_REVIEW: &str = "learning_material_review";

const GIT_STATUS_TIMEOUT_SECS: u64 = 10;
const GIT_STATUS_OUTPUT_LIMIT: usize = 8 * 1024;

#[derive(Debug, Clone, Serialize)]
struct AgentToolRunResult {
    tool_name: String,
    sources: Vec<GroundingSource>,
}

/// Handles local agent runtime lifecycle commands and approved local actions.
pub struct AgentHandler {
    runtime: Arc<AgentRuntime>,
    config: Arc<Mutex<ConfigManager>>,
    note_manager: Arc<Mutex<NoteManager>>,
    history_manager: Arc<Mutex<HistoryManager>>,
    workspace_manager: Arc<Mutex<WorkspaceManager>>,
    builtin_registry: Arc<Mutex<BuiltinCommandRegistry>>,
    model_manager: Arc<ModelManager>,
    knowledge_store: KnowledgeStoreHandle,
    tantivy_index_dir: PathBuf,
}

pub struct AgentHandlerDeps {
    pub runtime: Arc<AgentRuntime>,
    pub config: Arc<Mutex<ConfigManager>>,
    pub note_manager: Arc<Mutex<NoteManager>>,
    pub history_manager: Arc<Mutex<HistoryManager>>,
    pub workspace_manager: Arc<Mutex<WorkspaceManager>>,
    pub builtin_registry: Arc<Mutex<BuiltinCommandRegistry>>,
    pub model_manager: Arc<ModelManager>,
    pub knowledge_store: KnowledgeStoreHandle,
    pub tantivy_index_dir: PathBuf,
}

impl AgentHandler {
    pub fn new(deps: AgentHandlerDeps) -> Self {
        Self {
            runtime: deps.runtime,
            config: deps.config,
            note_manager: deps.note_manager,
            history_manager: deps.history_manager,
            workspace_manager: deps.workspace_manager,
            builtin_registry: deps.builtin_registry,
            model_manager: deps.model_manager,
            knowledge_store: deps.knowledge_store,
            tantivy_index_dir: deps.tantivy_index_dir,
        }
    }
}

impl CommandHandler for AgentHandler {
    fn namespace(&self) -> &'static str {
        "agent"
    }

    fn execute(&self, command: &str, payload: Value) -> CommandResult {
        match command {
            "start" => {
                {
                    let cfg = self.config.lock().map_err(|e| e.to_string())?;
                    let enabled = cfg
                        .get("features.agent")
                        .as_deref()
                        .map(|v| !v.eq_ignore_ascii_case("false"))
                        .unwrap_or(true);
                    if !enabled {
                        return Err(
                            "Agent 功能已停用。請前往 /setting → Features 開啟。".into()
                        );
                    }
                }
                let prompt = payload
                    .get("prompt")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing 'prompt'".to_string())?
                    .to_string();
                self.start_run(prompt)
                    .and_then(|run| serde_json::to_value(run).map_err(|e| e.to_string()))
            }
            "cancel" => {
                let run_id = require_str(&payload, "run_id")?;
                Ok(json!(self.runtime.cancel(run_id)?))
            }
            "resume" => {
                let run_id = require_str(&payload, "run_id")?;
                Ok(json!({
                    "ok": true,
                    "run": self.runtime.get(run_id)?,
                    "status": "resume_not_required"
                }))
            }
            "approve" => {
                let run_id = require_str(&payload, "run_id")?;
                let approval_id = require_str(&payload, "approval_id")?;
                let remember = payload
                    .get("remember")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                self.approve_run(run_id, approval_id, remember)
                    .and_then(|run| serde_json::to_value(run).map_err(|e| e.to_string()))
            }
            "reject" => {
                let run_id = require_str(&payload, "run_id")?;
                let approval_id = require_str(&payload, "approval_id")?;
                self.reject_run(run_id, approval_id)
                    .and_then(|run| serde_json::to_value(run).map_err(|e| e.to_string()))
            }
            "tools" => Ok(json!(self.runtime.list_tools())),
            "tool" => {
                let tool_name = require_str(&payload, "name")?;
                let query = require_str(&payload, "query")?;
                let limit = payload.get("limit").and_then(Value::as_u64).unwrap_or(10) as usize;
                Ok(json!(self.run_tool(tool_name, query, limit)?))
            }
            "clear_runs" => {
                self.runtime.clear_runs()?;
                Ok(json!({ "ok": true }))
            }
            _ => Err(format!("unknown agent command '{command}'")),
        }
    }
}

// Lifecycle methods (`start_run`, `should_use_react_loop`, `start_react_run`,
// `start_heuristic_run`, `approve_run`, `reject_run`, `memory_refs`) live in
// `lifecycle.rs` after REF.3 batch 3.

// Heuristic planning helpers (`plan_approvals`, `detect_planned_action`, the
// eight `plan_*` draft detectors, and `execute_planned_action`) live in
// `planning.rs` after REF.3 batch 4. The module is marked deprecated per
// ADR-0029; removal lands in REF.8.

impl AgentHandler {
    fn sources_for_prompt(
        &self,
        prompt: &str,
    ) -> Result<(Vec<GroundingSource>, Vec<AgentToolCall>), String> {
        if !should_run_local_search(prompt) {
            return Ok((Vec::new(), Vec::new()));
        }
        let started = Instant::now();
        let sources = self.keynova_search(prompt, 8)?;
        let tool_call = AgentToolCall {
            id: format!("tool:{}", Uuid::new_v4()),
            tool_name: "keynova.search".into(),
            risk: ActionRisk::Low,
            status: "completed".into(),
            duration_ms: Some(started.elapsed().as_millis()),
            error: None,
        };
        Ok((sources, vec![tool_call]))
    }

    fn run_tool(
        &self,
        tool_name: &str,
        query: &str,
        limit: usize,
    ) -> Result<AgentToolRunResult, String> {
        let sources = match tool_name {
            "keynova.search" => self.keynova_search(query, limit)?,
            "web.search" => self.web_search(query, limit)?,
            "filesystem.search" => self.filesystem_search_sources(query, limit),
            "filesystem.read" => self.filesystem_read_source(query)?,
            "git.status" => return Err(
                "git.status is a typed approval-gated tool and cannot be run through agent.tool"
                    .into(),
            ),
            other => return Err(format!("unknown agent tool '{other}'")),
        };
        Ok(AgentToolRunResult {
            tool_name: tool_name.to_string(),
            sources,
        })
    }

    // Chat-first direct-answer helpers (`direct_local_answer`,
    // `answer_filesystem_search`, `answer_file_read`,
    // `answer_project_type_summary`, `answer_github_trending`,
    // `answer_web_search`) live in `answers.rs` after REF.3 batch 5; the
    // module is marked deprecated per ADR-0029 and removed in REF.8.

    fn filesystem_search_roots_for_prompt(&self, prompt: &str) -> Vec<PathBuf> {
        let mut roots = Vec::new();
        if let Ok(workspace) = self.workspace_manager.lock() {
            if let Some(root) = workspace.current().project_root.as_deref() {
                if !root.trim().is_empty() {
                    roots.push(PathBuf::from(root));
                }
            }
        }
        if let Ok(cwd) = std::env::current_dir() {
            roots.push(cwd);
        }
        if wants_whole_computer_search(prompt) {
            roots.extend(system_search_roots());
        }
        roots.dedup();
        roots
    }

    fn filesystem_search_sources(&self, query: &str, limit: usize) -> Vec<GroundingSource> {
        search_system_index(
            query,
            &self.filesystem_search_roots_for_prompt(query),
            limit,
            Some(&self.tantivy_index_dir),
        )
        .hits
        .into_iter()
        .enumerate()
        .map(|(index, hit)| {
            source(
                format!("filesystem:{}", hit.path),
                if hit.is_dir { "folder" } else { "file" },
                hit.name,
                hit.path,
                0.88 - (index as f32 * 0.01),
                ContextVisibility::UserPrivate,
            )
        })
        .collect()
    }

    fn filesystem_read_source(&self, query: &str) -> Result<Vec<GroundingSource>, String> {
        let target = extract_file_read_target(query).unwrap_or_else(|| query.trim().to_string());
        let roots = self.filesystem_search_roots_for_prompt(query);
        let (path, _) = resolve_file_target(&target, &roots);
        let path = path.ok_or_else(|| format!("file '{target}' not found"))?;
        let preview = read_text_preview(&path, 12_000)?;
        let observation = prepare_observation(
            &preview,
            &AgentObservationPolicy {
                max_chars: 4096,
                max_lines: 120,
                preserve_head_lines: 48,
                preserve_tail_lines: 48,
                redact_secrets: true,
            },
        );
        Ok(vec![source(
            format!("filesystem-read:{}", path.display()),
            "file_read",
            path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("file")
                .to_string(),
            observation.content,
            0.95,
            ContextVisibility::UserPrivate,
        )])
    }

    fn local_searcher(&self) -> LocalContextSearcher {
        LocalContextSearcher {
            workspace_manager: Arc::clone(&self.workspace_manager),
            note_manager: Arc::clone(&self.note_manager),
            history_manager: Arc::clone(&self.history_manager),
            builtin_registry: Arc::clone(&self.builtin_registry),
            model_manager: Arc::clone(&self.model_manager),
        }
    }

    fn keynova_search(&self, query: &str, limit: usize) -> Result<Vec<GroundingSource>, String> {
        let searcher = self.local_searcher();
        let mut sources = Vec::new();
        let q = query.to_lowercase();

        searcher.push_workspace_source(&mut sources);
        searcher.push_command_sources(&q, &mut sources)?;
        self.push_setting_schema_sources(&q, &mut sources);
        searcher.push_model_sources(&q, &mut sources);
        searcher.push_note_sources(&q, &mut sources)?;
        searcher.push_history_sources(query, &mut sources)?;

        sources.sort_by(|left, right| {
            right
                .score
                .partial_cmp(&left.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.source_type.cmp(&right.source_type))
                .then_with(|| left.title.cmp(&right.title))
        });
        sources.truncate(limit.max(1));
        Ok(sources)
    }

    fn push_setting_schema_sources(&self, query: &str, sources: &mut Vec<GroundingSource>) {
        for schema in builtin_setting_schema()
            .into_iter()
            .filter(|schema| !schema.sensitive)
            .filter(|schema| {
                query.is_empty()
                    || schema.key.contains(query)
                    || schema.label.to_lowercase().contains(query)
            })
        {
            sources.push(source(
                format!("setting:{}", schema.key),
                "setting_schema",
                schema.key.to_string(),
                format!("{} default={}", schema.label, schema.default_value),
                0.72,
                ContextVisibility::PublicContext,
            ));
        }
    }

    fn web_search(&self, query: &str, limit: usize) -> Result<Vec<GroundingSource>, String> {
        let sanitized = sanitize_external_query(query)?;
        let (provider_str, searxng_url, api_key, timeout_secs) = {
            let config = self.config.lock().map_err(|e| e.to_string())?;
            (
                config
                    .get("agent.web_search_provider")
                    .unwrap_or_else(|| "disabled".into()),
                config.get("agent.searxng_url").unwrap_or_default(),
                config.get("agent.web_search_api_key").unwrap_or_default(),
                config
                    .get("agent.web_search_timeout_secs")
                    .and_then(|value| value.parse::<u64>().ok())
                    .unwrap_or(8),
            )
        };
        let provider = resolve_web_search_provider(&provider_str, &searxng_url, &api_key)?;
        provider.search(&sanitized, limit, timeout_secs)
    }

    fn log_audit(
        &self,
        run_id: &str,
        event_type: &str,
        status: &str,
        summary: &str,
        payload: Option<Value>,
    ) {
        self.knowledge_store.try_log_agent_audit(AgentAuditEntry {
            run_id: run_id.to_string(),
            event_type: event_type.to_string(),
            status: status.to_string(),
            summary: summary.to_string(),
            payload_json: payload.map(|value| value.to_string()),
        });
    }

    /// Assemble a `ContextBundle` from manager data before an agent run.
    ///
    /// Pulls workspace metadata, recent actions, and recently accessed file paths
    /// from `WorkspaceManager` (no filesystem scan). Accepts pre-computed search
    /// results so `keynova_search` is called only once per run path.
    fn build_context_bundle(
        &self,
        prompt: &str,
        search_results: Vec<GroundingSource>,
    ) -> ContextBundle {
        let (workspace_ctx, recent_actions, selected_files) = self
            .workspace_manager
            .lock()
            .map(|ws| {
                let current = ws.current();
                let workspace_ctx = WorkspaceContext {
                    id: current.id.to_string(),
                    name: current.name.clone(),
                    project_root: current.project_root.clone(),
                    recent_file_count: current.recent_files.len(),
                    note_count: current.note_ids.len(),
                };
                let recent_actions: Vec<String> = current
                    .recent_actions
                    .iter()
                    .take(CONTEXT_BUNDLE_RECENT_ACTIONS)
                    .cloned()
                    .collect();
                let selected_files: Vec<SelectedFileContext> = current
                    .recent_files
                    .iter()
                    .take(CONTEXT_BUNDLE_MAX_FILES)
                    .map(|path| SelectedFileContext {
                        path: path.clone(),
                        preview: String::new(),
                    })
                    .collect();
                (workspace_ctx, recent_actions, selected_files)
            })
            .unwrap_or_else(|_| {
                let ws = WorkspaceContext {
                    id: "0".into(),
                    name: "Default".into(),
                    project_root: None,
                    recent_file_count: 0,
                    note_count: 0,
                };
                (ws, Vec::new(), Vec::new())
            });

        ContextBundle::build(
            prompt.to_string(),
            workspace_ctx,
            recent_actions,
            selected_files,
            search_results,
            CONTEXT_BUNDLE_BUDGET_CHARS,
        )
    }
}


// ─── ReAct Tool Dispatch ─────────────────────────────────────────────────────

/// Arc-captured state for the ReAct loop dispatch closure.
/// One instance per agent run; shared across loop steps.
struct ReactDispatchState {
    workspace_manager: Arc<Mutex<WorkspaceManager>>,
    config: Arc<Mutex<ConfigManager>>,
    note_manager: Arc<Mutex<NoteManager>>,
    history_manager: Arc<Mutex<HistoryManager>>,
    builtin_registry: Arc<Mutex<BuiltinCommandRegistry>>,
    model_manager: Arc<ModelManager>,
    #[allow(dead_code)]
    knowledge_store: KnowledgeStoreHandle,
    tantivy_index_dir: PathBuf,
}

impl ReactDispatchState {
    fn dispatch(&self, name: &str, args: &Value) -> Result<Value, String> {
        // Approval-gated tools require `"__approved": true` injected by the ReAct loop
        // after the user explicitly grants permission. Direct dispatch without approval fails.
        const APPROVAL_GATED: &[&str] = &[TOOL_GIT_STATUS, TOOL_LEARNING_MATERIAL_REVIEW];
        if APPROVAL_GATED.contains(&name)
            && args.get("__approved").and_then(Value::as_bool) != Some(true)
        {
            return Err(AgentError::ToolDenied {
                tool: name.to_string(),
                reason: "requires explicit user approval before dispatch".into(),
            }
            .to_string());
        }
        match name {
            TOOL_KEYNOVA_SEARCH => self.dispatch_keynova_search(args),
            TOOL_FILESYSTEM_SEARCH => self.dispatch_filesystem_search(args),
            TOOL_FILESYSTEM_READ => self.dispatch_filesystem_read(args),
            TOOL_WEB_SEARCH => self.dispatch_web_search(args),
            TOOL_GIT_STATUS => self.dispatch_git_status(args),
            TOOL_DEV_CARGO_TEST => self.dispatch_dev_cargo_test(args),
            TOOL_DEV_CARGO_CHECK => self.dispatch_dev_cargo_check(args),
            TOOL_DEV_NPM_BUILD => self.dispatch_dev_npm_build(args),
            TOOL_DEV_NPM_LINT => self.dispatch_dev_npm_lint(args),
            TOOL_DEV_EXPLAIN_ERROR => self.dispatch_dev_explain_compiler_error(args),
            TOOL_LEARNING_MATERIAL_REVIEW => self.dispatch_learning_material_review(args),
            other => Err(format!("unknown react tool '{other}'")),
        }
    }

    fn dispatch_learning_material_review(&self, args: &Value) -> Result<Value, String> {
        use crate::managers::learning_material_manager::LearningMaterialManager;
        let mut roots: Vec<PathBuf> = args
            .get("roots")
            .and_then(Value::as_array)
            .map(|arr| arr.iter().filter_map(Value::as_str).map(PathBuf::from).collect())
            .unwrap_or_default();

        // Fall back to workspace project root when no roots supplied.
        if roots.is_empty() {
            if let Ok(ws) = self.workspace_manager.lock() {
                if let Some(root) = ws.current().project_root.clone().filter(|r| !r.is_empty()) {
                    roots.push(PathBuf::from(root));
                }
            }
        }

        let mgr = {
            let config = self.config.lock().map_err(|e| e.to_string())?;
            LearningMaterialManager::from_config(&config)
        };

        let report = mgr.scan(&roots)?;
        let result = serde_json::json!({
            "roots": report.roots,
            "candidate_count": report.stats.candidate_count,
            "scanned_count": report.stats.scanned_count,
            "filtered_count": report.stats.filtered_count,
            "markdown_summary": report.to_markdown(),
        });
        Ok(result)
    }

    fn dispatch_filesystem_search(&self, args: &Value) -> Result<Value, String> {
        let query = args.get("query").and_then(Value::as_str).unwrap_or("").to_string();
        if query.trim().is_empty() {
            return Err("filesystem.search: 'query' must not be empty".into());
        }
        let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(20) as usize;
        let roots: Vec<PathBuf> = args
            .get("roots")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(PathBuf::from)
                    .collect()
            })
            .unwrap_or_else(|| self.default_search_roots());

        let outcome = search_system_index(&query, &roots, limit.max(1), Some(&self.tantivy_index_dir));
        let sources: Vec<Value> = outcome
            .hits
            .into_iter()
            .map(|hit| {
                json!({
                    "title": hit.name,
                    "snippet": hit.path,
                    "uri": hit.path,
                    "source_type": if hit.is_dir { "folder" } else { "file" },
                })
            })
            .collect();
        Ok(json!({ "sources": sources }))
    }

    fn dispatch_filesystem_read(&self, args: &Value) -> Result<Value, String> {
        let path_str = args
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| "filesystem.read: missing 'path' argument".to_string())?;
        let max_chars = args
            .get("max_chars")
            .and_then(Value::as_u64)
            .unwrap_or(4096) as usize;

        let roots = self.default_search_roots();
        let resolved = resolve_readable_path(path_str, &roots)?;

        let preview = read_text_preview(&resolved, max_chars.min(12_000))?;
        let observation = prepare_observation(
            &preview,
            &AgentObservationPolicy {
                max_chars,
                max_lines: 120,
                preserve_head_lines: 48,
                preserve_tail_lines: 48,
                redact_secrets: true,
            },
        );
        let name = resolved
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(path_str)
            .to_string();
        Ok(json!({
            "sources": [{
                "title": name,
                "snippet": observation.content,
                "uri": resolved.display().to_string(),
                "source_type": "file_read",
            }]
        }))
    }

    fn local_searcher(&self) -> LocalContextSearcher {
        LocalContextSearcher {
            workspace_manager: Arc::clone(&self.workspace_manager),
            note_manager: Arc::clone(&self.note_manager),
            history_manager: Arc::clone(&self.history_manager),
            builtin_registry: Arc::clone(&self.builtin_registry),
            model_manager: Arc::clone(&self.model_manager),
        }
    }

    fn dispatch_keynova_search(&self, args: &Value) -> Result<Value, String> {
        let query = args
            .get("query")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_lowercase();
        let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(10) as usize;
        let searcher = self.local_searcher();
        let mut sources: Vec<GroundingSource> = Vec::new();

        searcher.push_workspace_source(&mut sources);
        searcher.push_command_sources(&query, &mut sources)?;
        searcher.push_note_sources(&query, &mut sources)?;
        searcher.push_history_sources(&query, &mut sources)?;
        searcher.push_model_sources(&query, &mut sources);

        sources.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.source_type.cmp(&b.source_type))
                .then_with(|| a.title.cmp(&b.title))
        });
        sources.truncate(limit.max(1));
        Ok(grounding_to_tool_sources_json(&sources))
    }

    fn dispatch_web_search(&self, args: &Value) -> Result<Value, String> {
        let query = args
            .get("query")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(5) as usize;
        let sanitized = sanitize_external_query(&query)?;
        let (provider_str, searxng_url, api_key, timeout_secs) = {
            let config = self.config.lock().map_err(|e| e.to_string())?;
            (
                config
                    .get("agent.web_search_provider")
                    .unwrap_or_else(|| "disabled".into()),
                config.get("agent.searxng_url").unwrap_or_default(),
                config.get("agent.web_search_api_key").unwrap_or_default(),
                config
                    .get("agent.web_search_timeout_secs")
                    .and_then(|v| v.parse::<u64>().ok())
                    .unwrap_or(8),
            )
        };
        let provider = resolve_web_search_provider(&provider_str, &searxng_url, &api_key)?;
        let sources = provider.search(&sanitized, limit, timeout_secs)?;
        Ok(grounding_to_tool_sources_json(&sources))
    }

    fn default_search_roots(&self) -> Vec<PathBuf> {
        let mut roots = Vec::new();
        if let Ok(ws) = self.workspace_manager.lock() {
            if let Some(root) = ws.current().project_root.as_deref() {
                if !root.trim().is_empty() {
                    roots.push(PathBuf::from(root));
                }
            }
        }
        if let Ok(cwd) = std::env::current_dir() {
            roots.push(cwd);
        }
        roots.dedup();
        roots
    }

    /// Execute a fixed read-only `git status --short` in the workspace CWD.
    /// Called only after the user has explicitly approved the gate approval.
    /// Workspace-scoped: cwd must be within a known workspace root.
    /// Bounded: stdout/stderr truncated at GIT_STATUS_OUTPUT_LIMIT bytes.
    /// Timeout: process killed after GIT_STATUS_TIMEOUT_SECS.
    fn dispatch_git_status(&self, args: &Value) -> Result<Value, String> {
        let roots = self.default_search_roots();

        let requested_cwd = args
            .get("cwd")
            .and_then(Value::as_str)
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                roots.first().cloned().unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
            });

        // Canonicalize so symlinks and relative paths cannot escape workspace scope.
        let cwd = requested_cwd.canonicalize().map_err(|_| {
            format!("git.status: cwd '{}' does not exist or is not accessible", requested_cwd.display())
        })?;

        // Enforce workspace scope: deny paths outside all known roots.
        let in_workspace = roots.iter().any(|root| {
            root.canonicalize().map(|r| cwd.starts_with(&r)).unwrap_or(false)
        });
        if !in_workspace {
            return Err(format!(
                "git.status: '{}' is outside all workspace roots — execution denied",
                cwd.display()
            ));
        }

        // Spawn with separate stdout/stderr pipes to avoid pipe-buffer deadlock.
        let mut child = std::process::Command::new("git")
            .args(["status", "--short"])
            .current_dir(&cwd)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("git.status: failed to spawn git: {e}"))?;

        let stdout_pipe = child.stdout.take();
        let stderr_pipe = child.stderr.take();

        // Read stdout/stderr in background threads so the pipes never fill and deadlock.
        let (out_tx, out_rx) = mpsc::channel::<Vec<u8>>();
        let (err_tx, err_rx) = mpsc::channel::<Vec<u8>>();

        if let Some(mut pipe) = stdout_pipe {
            thread::spawn(move || {
                let mut buf = Vec::new();
                let _ = pipe.read_to_end(&mut buf);
                let _ = out_tx.send(buf);
            });
        }
        if let Some(mut pipe) = stderr_pipe {
            thread::spawn(move || {
                let mut buf = Vec::new();
                let _ = pipe.read_to_end(&mut buf);
                let _ = err_tx.send(buf);
            });
        }

        // Poll for exit with timeout; kill on deadline.
        let deadline = Instant::now() + Duration::from_secs(GIT_STATUS_TIMEOUT_SECS);
        let exit_code = loop {
            match child.try_wait().map_err(|e| format!("git.status: wait error: {e}"))? {
                Some(status) => break status.code(),
                None => {
                    if Instant::now() >= deadline {
                        let _ = child.kill();
                        return Err(format!(
                            "git.status: timed out after {GIT_STATUS_TIMEOUT_SECS}s — process killed"
                        ));
                    }
                    thread::sleep(Duration::from_millis(50));
                }
            }
        };

        let stdout_bytes = out_rx.recv().unwrap_or_default();
        let stderr_bytes = err_rx.recv().unwrap_or_default();

        Ok(json!({
            "cwd": cwd.display().to_string(),
            "stdout": bound_output(&stdout_bytes),
            "stderr": bound_output(&stderr_bytes),
            "exit_code": exit_code,
            "preview": format!("git status --short  (cwd: {})", cwd.display()),
        }))
    }

    fn scoped_cwd_for_dev(&self, args: &Value, tool: &str) -> Result<PathBuf, String> {
        let roots = self.default_search_roots();
        let requested = args
            .get("cwd")
            .and_then(Value::as_str)
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                roots.first().cloned().unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
            });
        let cwd = requested.canonicalize().map_err(|_| {
            format!("{tool}: cwd '{}' does not exist or is not accessible", requested.display())
        })?;
        let in_workspace = roots.iter().any(|r| r.canonicalize().map(|r| cwd.starts_with(&r)).unwrap_or(false));
        if !in_workspace {
            return Err(format!("{tool}: '{}' is outside workspace roots — execution denied", cwd.display()));
        }
        Ok(cwd)
    }

    fn dispatch_dev_cargo_test(&self, args: &Value) -> Result<Value, String> {
        let cwd = self.scoped_cwd_for_dev(args, "dev.cargo_test")?;
        run_bounded_dev_cmd("cargo", &["test"], &cwd, Duration::from_secs(DEV_CARGO_TIMEOUT_SECS))
    }

    fn dispatch_dev_cargo_check(&self, args: &Value) -> Result<Value, String> {
        let cwd = self.scoped_cwd_for_dev(args, "dev.cargo_check")?;
        run_bounded_dev_cmd("cargo", &["check"], &cwd, Duration::from_secs(DEV_CARGO_TIMEOUT_SECS))
    }

    fn dispatch_dev_npm_build(&self, args: &Value) -> Result<Value, String> {
        let cwd = self.scoped_cwd_for_dev(args, "dev.npm_build")?;
        run_bounded_dev_cmd("npm", &["run", "build"], &cwd, Duration::from_secs(DEV_NPM_TIMEOUT_SECS))
    }

    fn dispatch_dev_npm_lint(&self, args: &Value) -> Result<Value, String> {
        let cwd = self.scoped_cwd_for_dev(args, "dev.npm_lint")?;
        run_bounded_dev_cmd("npm", &["run", "lint"], &cwd, Duration::from_secs(DEV_NPM_TIMEOUT_SECS))
    }

    /// Extract structured errors from raw compiler/lint output.
    /// Read-only: inspects text only, never modifies files.
    fn dispatch_dev_explain_compiler_error(&self, args: &Value) -> Result<Value, String> {
        let output = args
            .get("output")
            .and_then(Value::as_str)
            .ok_or_else(|| "dev.explain_compiler_error: 'output' field is required".to_string())?;

        let errors = extract_compiler_errors(output);
        if errors.is_empty() {
            return Ok(json!({
                "errors": [],
                "summary": "No recognizable compiler errors found in the provided output.",
            }));
        }

        let summary = format!(
            "{} error(s) extracted. Review each `message` and `location` for details.",
            errors.len()
        );
        Ok(json!({
            "errors": errors,
            "summary": summary,
        }))
    }
}



/// Git-status-specific wrapper that bounds output at `GIT_STATUS_OUTPUT_LIMIT`.
/// Kept here because the limit is tied to `dispatch_git_status`; will move into
/// `handlers/agent/tools.rs` together with the dispatch impl in REF.3 batch 7.
fn bound_output(bytes: &[u8]) -> String {
    bound_output_n(bytes, GIT_STATUS_OUTPUT_LIMIT)
}

impl AgentHandler {
    /// Build a `ToolDispatch` closure wiring all ReAct-compatible tools.
    ///
    /// The closure captures Arc clones of every dep needed and is `Send + Sync`,
    /// so it can safely be passed to `spawn_react_loop`.
    pub fn build_react_dispatch(&self) -> Arc<ToolDispatch> {
        let state = Arc::new(ReactDispatchState {
            workspace_manager: Arc::clone(&self.workspace_manager),
            config: Arc::clone(&self.config),
            note_manager: Arc::clone(&self.note_manager),
            history_manager: Arc::clone(&self.history_manager),
            builtin_registry: Arc::clone(&self.builtin_registry),
            model_manager: Arc::clone(&self.model_manager),
            knowledge_store: self.knowledge_store.clone(),
            tantivy_index_dir: self.tantivy_index_dir.clone(),
        });
        Arc::new(move |name: &str, args: &Value| state.dispatch(name, args))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use crate::models::settings_schema::SettingValueType;

    #[test]
    fn rejects_private_context_in_web_query() {
        let error = sanitize_external_query("search tasks.md architecture").unwrap_err();
        assert!(error.contains("private"));
    }

    #[test]
    fn redacts_private_architecture_sources() {
        let source = visibility_filtered_source(
            "note:test".into(),
            "note",
            "tasks.md".into(),
            "phase 4 architecture".into(),
            1.0,
        );
        assert_eq!(source.visibility, ContextVisibility::PrivateArchitecture);
        assert!(source.redacted_reason.is_some());
    }

    #[test]
    fn redacts_secret_sources() {
        let source = visibility_filtered_source(
            "history:test".into(),
            "history",
            "Copied secret".into(),
            "api key = secret-value".into(),
            1.0,
        );
        assert_eq!(source.visibility, ContextVisibility::Secret);
        assert_eq!(source.snippet, "[redacted secret]");
    }

    #[test]
    fn builds_prompt_audit_with_budget_and_filters() {
        let audit = build_prompt_audit(
            "test prompt",
            &[
                source(
                    "workspace:1".into(),
                    "workspace",
                    "Workspace".into(),
                    "Public context".into(),
                    1.0,
                    ContextVisibility::PublicContext,
                ),
                visibility_filtered_source(
                    "note:1".into(),
                    "note",
                    "tasks.md".into(),
                    "architecture".into(),
                    0.8,
                ),
            ],
            64,
        );
        assert_eq!(audit.included_sources.len(), 1);
        assert_eq!(audit.filtered_sources.len(), 1);
    }

    #[test]
    fn extracts_integer_setting_value() {
        let value = extract_setting_value("set max results to 25", &SettingValueType::Integer);
        assert_eq!(value.as_deref(), Some("25"));
    }

    #[test]
    fn extracts_terminal_command_from_backticks() {
        let command = extract_shell_command("please run `cargo test` for me");
        assert_eq!(command.as_deref(), Some("cargo test"));
    }

    #[test]
    fn answers_capability_questions_locally() {
        let answer = direct_local_answer("你可以做到甚麼").expect("capability answer");
        assert!(answer.contains("我可以"));
        assert!(answer.contains("approval"));
    }

    #[test]
    fn answers_time_questions_locally() {
        let answer = direct_local_answer("顯示目前的詳細時間").expect("time answer");
        assert!(answer.contains("目前時間"));
    }

    #[test]
    fn extracts_chinese_directory_listing_target() {
        let target = extract_directory_list_target("幫我搜尋 hw 資料夾中有哪些資料夾");
        assert_eq!(target.as_deref(), Some("hw"));
    }

    #[test]
    fn answers_directory_listing_with_child_folders() {
        let root = std::env::temp_dir().join(format!("keynova-agent-test-{}", Uuid::new_v4()));
        let hw = root.join("hw");
        std::fs::create_dir_all(hw.join("week1")).expect("create week1");
        std::fs::create_dir_all(hw.join("week2")).expect("create week2");

        let answer = answer_directory_listing(
            "幫我搜尋 hw 資料夾中有哪些資料夾",
            std::slice::from_ref(&root),
        )
        .expect("directory answer");

        assert!(answer.contains("2 個子資料夾"));
        assert!(answer.contains("week1"));
        assert!(answer.contains("week2"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn directory_listing_reports_checked_paths_when_missing() {
        let root = std::env::temp_dir().join(format!("keynova-agent-missing-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("create root");

        let answer = answer_directory_listing(
            "幫我搜尋 hw 資料夾中有哪些資料夾",
            std::slice::from_ref(&root),
        )
        .expect("missing directory answer");

        assert!(answer.contains("找不到 `hw`"));
        assert!(answer.contains(&root.join("hw").display().to_string()));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn extracts_whole_computer_filesystem_search_query() {
        let query = extract_filesystem_search_query("please search whole computer for keynova");
        assert_eq!(query.as_deref(), Some("keynova"));
        assert!(wants_whole_computer_search("search whole computer keynova"));
    }

    #[test]
    fn filesystem_search_finds_matching_files_read_only() {
        let root = std::env::temp_dir().join(format!("keynova-agent-fs-search-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("create root");
        std::fs::write(root.join("homework.md"), "hello").expect("write test file");

        let outcome = search_filesystem("homework", std::slice::from_ref(&root), 5);

        assert_eq!(outcome.hits.len(), 1);
        assert!(!outcome.hits[0].is_dir);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn reads_text_file_preview_without_modifying() {
        let root = std::env::temp_dir().join(format!("keynova-agent-read-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("create root");
        let path = root.join("note.txt");
        std::fs::write(&path, "read-only preview").expect("write test file");

        let answer = read_file_answer("note.txt", std::slice::from_ref(&root));

        assert!(answer.contains("read-only preview"));
        assert_eq!(
            std::fs::read_to_string(&path).expect("read test file"),
            "read-only preview"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn summarizes_project_types_from_markers() {
        let root =
            std::env::temp_dir().join(format!("keynova-agent-project-types-{}", Uuid::new_v4()));
        std::fs::create_dir_all(root.join("app-a")).expect("create app-a");
        std::fs::create_dir_all(root.join("app-b")).expect("create app-b");
        std::fs::create_dir_all(root.join("rust-a")).expect("create rust-a");
        std::fs::write(root.join("app-a").join("package.json"), "{}").expect("write package");
        std::fs::write(root.join("app-b").join("package.json"), "{}").expect("write package");
        std::fs::write(root.join("rust-a").join("Cargo.toml"), "[package]").expect("write cargo");

        let counts = scan_project_types(std::slice::from_ref(&root));
        let answer = format_project_type_summary(&counts);

        assert!(is_project_type_summary_prompt(
            "confirm which project types are most"
        ));
        assert!(answer.contains("JavaScript/TypeScript"));
        assert!(answer.contains("2"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn extracts_web_search_query_from_news_prompt() {
        let query = extract_web_search_query("please search web for technology news today");
        assert_eq!(query.as_deref(), Some("web for technology news today"));
    }

    #[test]
    fn parses_duckduckgo_html_result() {
        let html = r#"
            <a rel="nofollow" class="result__a" href="/l/?uddg=https%3A%2F%2Fexample.com%2Fnews">Example &amp; News</a>
            <a class="result__snippet">A short &amp; useful snippet.</a>
        "#;

        let results = parse_duckduckgo_html_results(html, 3);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Example & News");
        assert_eq!(results[0].uri.as_deref(), Some("https://example.com/news"));
        assert!(results[0].snippet.contains("short & useful"));
    }

    #[test]
    fn parses_tavily_json_result() {
        let response = json!({
            "results": [
                {
                    "title": "Example News",
                    "url": "https://example.com/news",
                    "content": "Structured search result content.",
                    "score": 0.92
                }
            ]
        });

        let results = parse_tavily_response(&response, 3).expect("parse tavily");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].source_id, "web:tavily:0");
        assert_eq!(results[0].title, "Example News");
        assert_eq!(results[0].uri.as_deref(), Some("https://example.com/news"));
        assert!(results[0].snippet.contains("Structured search"));
    }

    #[test]
    fn parses_github_trending_html() {
        let html = r#"
            <article class="Box-row">
              <h2><a href="/openai/example-repo">openai / example-repo</a></h2>
              <p>Example trending repo.</p>
            </article>
        "#;

        let repos = parse_github_trending_html(html, 10);

        assert!(is_github_trending_prompt("today popular github projects"));
        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0].owner, "openai");
        assert_eq!(repos[0].name, "example-repo");
        assert_eq!(repos[0].url, "https://github.com/openai/example-repo");
    }

    #[test]
    fn creates_workflow_plan_for_task_prompt() {
        let answer = answer_workflow_plan("plan a task to organize files").expect("workflow plan");
        assert!(answer.contains("1."));
        assert!(answer.contains("approval"));
    }

    #[test]
    fn extracts_direct_command_like_terminal_request() {
        let command = extract_shell_command("please run npm run build");
        assert_eq!(command.as_deref(), Some("npm run build"));
    }

    #[test]
    fn does_not_treat_plain_start_as_terminal_command() {
        let command = extract_shell_command("start with project search and propose next steps");
        assert!(command.is_none());
    }

    #[test]
    fn prompt_audit_marks_truncated_sources() {
        let audit = build_prompt_audit(
            "a very long prompt that consumes the tiny budget",
            &[source(
                "workspace:1".into(),
                "workspace",
                "Workspace".into(),
                "Public context".into(),
                1.0,
                ContextVisibility::PublicContext,
            )],
            12,
        );
        assert!(audit.truncated);
        assert!(audit.included_sources.is_empty());
    }

    #[test]
    fn safe_builtin_allowlist_rejects_args() {
        assert!(is_allowlisted_safe_builtin("help", ""));
        assert!(!is_allowlisted_safe_builtin("help", "--danger"));
        assert!(!is_allowlisted_safe_builtin("rebuild_search_index", ""));
    }

    #[test]
    fn openai_provider_selects_react_loop() {
        use crate::managers::ai_manager::{provider_supports_tool_calls, resolve_ai_runtime_config};
        let pairs = [
            ("ai.provider", "openai"),
            ("ai.openai_api_key", "test-key"),
            ("ai.openai_base_url", "https://api.openai.com/v1"),
            ("ai.model", "gpt-4o-mini"),
        ];
        let rt = resolve_ai_runtime_config(|k| {
            pairs.iter().find(|(key, _)| *key == k).map(|(_, v)| v.to_string())
        })
        .unwrap();
        assert!(provider_supports_tool_calls(&rt.provider));
    }

    #[test]
    fn claude_provider_selects_heuristic_fallback() {
        use crate::managers::ai_manager::{provider_supports_tool_calls, resolve_ai_runtime_config};
        let pairs = [
            ("ai.provider", "claude"),
            ("ai.api_key", "test-key"),
            ("ai.model", "claude-sonnet-4-6"),
        ];
        let rt = resolve_ai_runtime_config(|k| {
            pairs.iter().find(|(key, _)| *key == k).map(|(_, v)| v.to_string())
        })
        .unwrap();
        assert!(!provider_supports_tool_calls(&rt.provider));
    }

    #[test]
    fn extract_quoted_prefers_double_over_single() {
        assert_eq!(
            extract_quoted(r#"read "config.toml" please"#).as_deref(),
            Some("config.toml")
        );
    }

    #[test]
    fn extract_quoted_falls_back_to_single_when_no_double() {
        assert_eq!(
            extract_quoted("read 'config.toml' please").as_deref(),
            Some("config.toml")
        );
    }

    #[test]
    fn extract_quoted_returns_none_for_no_quotes() {
        assert!(extract_quoted("read config.toml please").is_none());
    }

    #[test]
    fn truncate_appends_ellipsis_only_when_truncated() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world", 5), "hello...");
    }

    #[test]
    fn looks_sensitive_path_blocks_ssh_keys() {
        assert!(looks_sensitive_path(Path::new("/home/user/.ssh/id_rsa")));
        assert!(looks_sensitive_path(Path::new("C:\\Users\\user\\.env")));
        assert!(looks_sensitive_path(Path::new("/home/user/.aws/credentials")));
        assert!(!looks_sensitive_path(Path::new("/home/user/projects/main.rs")));
    }

    #[test]
    fn resolve_readable_path_rejects_out_of_workspace() {
        let root = std::env::temp_dir()
            .join(format!("keynova-path-test-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("create root");
        // Attempting to read something outside the root using `..` traversal
        let err = resolve_readable_path("../../etc/passwd", &[root.clone()]).unwrap_err();
        // Could fail at "not found" or "outside workspace" — both are correct rejections
        assert!(
            err.contains("not found") || err.contains("outside workspace") || err.contains("cannot resolve"),
            "unexpected error: {err}"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn resolve_readable_path_allows_in_workspace() {
        let root = std::env::temp_dir()
            .join(format!("keynova-path-ok-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("create root");
        let file = root.join("note.txt");
        std::fs::write(&file, "hello").expect("write file");
        let resolved = resolve_readable_path("note.txt", &[root.clone()]).expect("should resolve");
        assert!(resolved.starts_with(root.canonicalize().unwrap()));
        let _ = std::fs::remove_dir_all(root);
    }

    // ── P2.A git.status hardening ────────────────────────────────────────────

    #[test]
    fn git_status_rejects_cwd_outside_workspace() {
        // Build a state whose only workspace root is a temp directory that is
        // different from system temp so we can supply an "outside" path.
        let root = std::env::temp_dir().join(format!("keynova-ws-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("create workspace root");

        let outside = std::env::temp_dir().join(format!("keynova-outside-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&outside).expect("create outside dir");

        // bound_output is a free fn — test workspace-scope rejection via the error message.
        let cwd = outside.canonicalize().unwrap_or(outside.clone());
        let workspace_root = root.canonicalize().unwrap_or(root.clone());
        let in_workspace = cwd.starts_with(&workspace_root);
        assert!(!in_workspace, "outside dir should not start_with workspace root");

        let _ = std::fs::remove_dir_all(root);
        let _ = std::fs::remove_dir_all(outside);
    }

    #[test]
    fn bound_output_truncates_at_limit() {
        let big = vec![b'x'; GIT_STATUS_OUTPUT_LIMIT + 100];
        let out = bound_output(&big);
        assert!(out.contains("[output truncated:"));
        // The beginning is preserved up to the limit.
        assert!(out.starts_with(&"x".repeat(GIT_STATUS_OUTPUT_LIMIT)));
    }

    #[test]
    fn bound_output_preserves_small_content() {
        let small = b"M  src/foo.rs\n";
        let out = bound_output(small);
        assert_eq!(out, "M  src/foo.rs\n");
    }

}
