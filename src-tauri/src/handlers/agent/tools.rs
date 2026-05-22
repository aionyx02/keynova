//! ReAct tool dispatch for `AgentHandler`.
//!
//! Owns `ReactDispatchState` (the Arc-captured state passed to each ReAct
//! loop), the `build_react_dispatch` constructor on `AgentHandler`, and every
//! `dispatch_*` method (keynova_search / filesystem_search / filesystem_read /
//! web_search / git_status / dev_cargo_test / dev_cargo_check / dev_npm_build
//! / dev_npm_lint / dev_explain_compiler_error / learning_material_review).
//!
//! Approval-gated tools require `args["__approved"] == true`, which the ReAct
//! loop injects only after the user explicitly grants permission for a
//! pending approval entry.

use std::io::Read;
use std::path::PathBuf;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use crate::core::agent_observation::{prepare_observation, AgentObservationPolicy};
use crate::core::agent_runtime::ToolDispatch;
use crate::core::config_manager::ConfigManager;
use crate::core::dev_runner::{
    bound_output_n, extract_compiler_errors, run_bounded_dev_cmd, DEV_CARGO_TIMEOUT_SECS,
    DEV_NPM_TIMEOUT_SECS,
};
use crate::core::local_context::LocalContextSearcher;
use crate::core::{BuiltinCommandRegistry, KnowledgeStoreHandle};
use crate::managers::{
    history_manager::HistoryManager, model_manager::ModelManager, note_manager::NoteManager,
    system_indexer::search_system_index, workspace_manager::WorkspaceManager,
};
use crate::models::agent::{AgentError, GroundingSource};

use super::filesystem::read_text_preview;
use super::formatting::grounding_to_tool_sources_json;
use super::safety::{resolve_readable_path, sanitize_external_query};
use super::web::resolve_web_search_provider;
use super::AgentHandler;

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
pub(super) const GIT_STATUS_OUTPUT_LIMIT: usize = 8 * 1024;

/// Git-status-specific wrapper that bounds output at `GIT_STATUS_OUTPUT_LIMIT`.
pub(super) fn bound_output(bytes: &[u8]) -> String {
    bound_output_n(bytes, GIT_STATUS_OUTPUT_LIMIT)
}

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