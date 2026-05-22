//! Grounding-source aggregation, IPC tool helpers, audit logging, and
//! `ContextBundle` assembly for `AgentHandler`.
//!
//! Owns the methods that are still needed by both the lifecycle (heuristic /
//! ReAct) path and the ReAct tool dispatcher:
//!   - `sources_for_prompt`, `run_tool`
//!   - `filesystem_search_roots_for_prompt`, `filesystem_search_sources`,
//!     `filesystem_read_source`
//!   - `local_searcher` (handler-side constructor), `keynova_search`,
//!     `push_setting_schema_sources`, `web_search`
//!   - `log_audit`, `build_context_bundle`

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use serde_json::Value;
use uuid::Uuid;

use crate::core::agent_observation::{prepare_observation, AgentObservationPolicy};
use crate::core::knowledge_store::AgentAuditEntry;
use crate::core::local_context::LocalContextSearcher;
use crate::managers::system_indexer::search_system_index;
use crate::models::action::ActionRisk;
use crate::models::agent::{AgentToolCall, ContextVisibility, GroundingSource};
use crate::models::context_bundle::{ContextBundle, SelectedFileContext, WorkspaceContext};
use crate::models::settings_schema::builtin_setting_schema;

use super::filesystem::{extract_file_read_target, read_text_preview, resolve_file_target};
use super::formatting::source;
use super::intent::{should_run_local_search, system_search_roots, wants_whole_computer_search};
use super::safety::sanitize_external_query;
use super::web::resolve_web_search_provider;
use super::{
    AgentHandler, AgentToolRunResult, CONTEXT_BUNDLE_BUDGET_CHARS, CONTEXT_BUNDLE_MAX_FILES,
    CONTEXT_BUNDLE_RECENT_ACTIONS,
};

impl AgentHandler {
    pub(super) fn sources_for_prompt(
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

    pub(super) fn run_tool(
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

    pub(super) fn filesystem_search_roots_for_prompt(&self, prompt: &str) -> Vec<PathBuf> {
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

    pub(super) fn web_search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<GroundingSource>, String> {
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

    pub(super) fn log_audit(
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
    pub(super) fn build_context_bundle(
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