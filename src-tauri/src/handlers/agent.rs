use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::{json, Value};

use crate::core::config_manager::ConfigManager;
use crate::core::{AgentRuntime, BuiltinCommandRegistry, CommandHandler, CommandResult};
use crate::managers::{
    history_manager::HistoryManager, model_manager::ModelManager, note_manager::NoteManager,
    workspace_manager::WorkspaceManager,
};
use crate::models::action::ActionRisk;
use crate::models::agent::{AgentToolCall, ContextVisibility, GroundingSource};
use crate::models::settings_schema::builtin_setting_schema;

#[derive(Debug, Clone, Serialize)]
struct AgentToolRunResult {
    tool_name: String,
    sources: Vec<GroundingSource>,
}

/// Handles local agent runtime lifecycle commands and read-only tool calls.
pub struct AgentHandler {
    runtime: Arc<AgentRuntime>,
    config: Arc<Mutex<ConfigManager>>,
    note_manager: Arc<Mutex<NoteManager>>,
    history_manager: Arc<Mutex<HistoryManager>>,
    workspace_manager: Arc<Mutex<WorkspaceManager>>,
    builtin_registry: Arc<Mutex<BuiltinCommandRegistry>>,
    model_manager: Arc<ModelManager>,
}

impl AgentHandler {
    pub fn new(
        runtime: Arc<AgentRuntime>,
        config: Arc<Mutex<ConfigManager>>,
        note_manager: Arc<Mutex<NoteManager>>,
        history_manager: Arc<Mutex<HistoryManager>>,
        workspace_manager: Arc<Mutex<WorkspaceManager>>,
        builtin_registry: Arc<Mutex<BuiltinCommandRegistry>>,
        model_manager: Arc<ModelManager>,
    ) -> Self {
        Self {
            runtime,
            config,
            note_manager,
            history_manager,
            workspace_manager,
            builtin_registry,
            model_manager,
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
                let prompt = payload
                    .get("prompt")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing 'prompt'".to_string())?
                    .to_string();
                let (sources, tool_calls) = self.sources_for_prompt(&prompt)?;
                Ok(json!(self
                    .runtime
                    .start_with_context(prompt, sources, tool_calls)?))
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
                Ok(json!({
                    "ok": true,
                    "run_id": run_id,
                    "status": "approval_recorded"
                }))
            }
            "reject" => {
                let run_id = require_str(&payload, "run_id")?;
                Ok(json!({
                    "ok": true,
                    "run_id": run_id,
                    "status": "approval_rejected"
                }))
            }
            "tools" => Ok(json!(self.runtime.list_tools())),
            "tool" => {
                let tool_name = require_str(&payload, "name")?;
                let query = require_str(&payload, "query")?;
                let limit = payload.get("limit").and_then(Value::as_u64).unwrap_or(10) as usize;
                Ok(json!(self.run_tool(tool_name, query, limit)?))
            }
            _ => Err(format!("unknown agent command '{command}'")),
        }
    }
}

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
            id: format!("tool:{}", uuid::Uuid::new_v4()),
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
            other => return Err(format!("unknown agent tool '{other}'")),
        };
        Ok(AgentToolRunResult {
            tool_name: tool_name.to_string(),
            sources,
        })
    }

    fn keynova_search(&self, query: &str, limit: usize) -> Result<Vec<GroundingSource>, String> {
        let mut sources = Vec::new();
        let q = query.to_lowercase();

        self.push_workspace_source(&mut sources);
        self.push_command_sources(&q, &mut sources)?;
        self.push_setting_schema_sources(&q, &mut sources);
        self.push_model_sources(&q, &mut sources);
        self.push_note_sources(&q, &mut sources)?;
        self.push_history_sources(query, &mut sources)?;

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

    fn push_workspace_source(&self, sources: &mut Vec<GroundingSource>) {
        let Ok(workspace) = self.workspace_manager.lock() else {
            return;
        };
        let current = workspace.current();
        let snippet = format!(
            "name={}, mode={}, panel={}, recent_files={}",
            current.name,
            current.mode,
            current.panel.as_deref().unwrap_or("none"),
            current.recent_files.len()
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

    fn push_command_sources(
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

    fn push_model_sources(&self, query: &str, sources: &mut Vec<GroundingSource>) {
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

    fn push_note_sources(
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

    fn push_history_sources(
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

    fn web_search(&self, query: &str, limit: usize) -> Result<Vec<GroundingSource>, String> {
        let sanitized = sanitize_external_query(query)?;
        let (provider, searxng_url, timeout_secs) = {
            let config = self.config.lock().map_err(|e| e.to_string())?;
            (
                config
                    .get("agent.web_search_provider")
                    .unwrap_or_else(|| "disabled".into()),
                config.get("agent.searxng_url").unwrap_or_default(),
                config
                    .get("agent.web_search_timeout_secs")
                    .and_then(|value| value.parse::<u64>().ok())
                    .unwrap_or(8),
            )
        };

        match provider.as_str() {
            "searxng" => search_searxng(&searxng_url, &sanitized, limit, timeout_secs),
            "disabled" | "" => Err(
                "web.search provider is disabled; set agent.web_search_provider=searxng and agent.searxng_url".into(),
            ),
            other => Err(format!("web.search provider '{other}' is not supported yet")),
        }
    }
}

fn search_searxng(
    base_url: &str,
    query: &str,
    limit: usize,
    timeout_secs: u64,
) -> Result<Vec<GroundingSource>, String> {
    if base_url.trim().is_empty() {
        return Err("agent.searxng_url is required for web.search".into());
    }
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(timeout_secs.max(1)))
        .build()
        .map_err(|e| e.to_string())?;
    let url = format!("{}/search", base_url.trim_end_matches('/'));
    let response = client
        .get(url)
        .query(&[("q", query), ("format", "json")])
        .send()
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .map_err(|e| e.to_string())?;
    let results = response
        .get("results")
        .and_then(Value::as_array)
        .ok_or_else(|| "searxng response missing results".to_string())?;
    Ok(results
        .iter()
        .take(limit.max(1))
        .enumerate()
        .map(|(index, item)| {
            let title = item
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or("Untitled")
                .to_string();
            let url = item
                .get("url")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned);
            let snippet = item
                .get("content")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            GroundingSource {
                source_id: format!("web:searxng:{index}"),
                source_type: "web".into(),
                title,
                snippet: truncate(&snippet, 240),
                uri: url,
                score: 1.0 - (index as f32 * 0.03),
                visibility: ContextVisibility::PublicContext,
                redacted_reason: None,
            }
        })
        .collect())
}

fn should_run_local_search(prompt: &str) -> bool {
    let q = prompt.to_lowercase();
    [
        "keynova", "search", "find", "model", "note", "history", "找", "搜尋", "模型", "筆記",
        "紀錄",
    ]
    .iter()
    .any(|needle| q.contains(needle))
}

fn sanitize_external_query(query: &str) -> Result<String, String> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Err("web.search query is empty".into());
    }
    let lower = trimmed.to_lowercase();
    for denied in [
        "claude.md",
        "tasks.md",
        "memory.md",
        "decisions.md",
        "skill.md",
        "private_architecture",
        "secret",
        "api_key",
        "架構",
    ] {
        if lower.contains(denied) {
            return Err(format!(
                "web.search query contains private or secret context term '{denied}'"
            ));
        }
    }
    Ok(trimmed.to_string())
}

fn visibility_filtered_source(
    source_id: String,
    source_type: &str,
    title: String,
    snippet: String,
    score: f32,
) -> GroundingSource {
    let combined = format!("{title} {snippet}").to_lowercase();
    if [
        "claude.md",
        "tasks.md",
        "memory.md",
        "decisions.md",
        "skill.md",
        "private_architecture",
        "架構",
    ]
    .iter()
    .any(|needle| combined.contains(needle))
    {
        return GroundingSource {
            source_id,
            source_type: source_type.into(),
            title,
            snippet: "[redacted private architecture context]".into(),
            uri: None,
            score,
            visibility: ContextVisibility::PrivateArchitecture,
            redacted_reason: Some("private_architecture".into()),
        };
    }
    source(
        source_id,
        source_type,
        title,
        snippet,
        score,
        ContextVisibility::UserPrivate,
    )
}

fn source(
    source_id: String,
    source_type: &str,
    title: String,
    snippet: String,
    score: f32,
    visibility: ContextVisibility,
) -> GroundingSource {
    GroundingSource {
        source_id,
        source_type: source_type.into(),
        title,
        snippet: truncate(&snippet, 240),
        uri: None,
        score,
        visibility,
        redacted_reason: None,
    }
}

fn truncate(value: &str, max_chars: usize) -> String {
    let mut out = value.chars().take(max_chars).collect::<String>();
    if value.chars().count() > max_chars {
        out.push_str("...");
    }
    out
}

fn require_str<'a>(payload: &'a Value, key: &str) -> Result<&'a str, String> {
    payload
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("missing or empty '{key}'"))
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
