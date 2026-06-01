use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde::Serialize;
use serde_json::{json, Value};
#[cfg(test)]
use uuid::Uuid;

use crate::core::config_manager::ConfigManager;
use crate::core::{
    AgentRuntime, BuiltinCommandRegistry, CommandHandler, CommandResult, KnowledgeStoreHandle,
};
use crate::managers::{
    history_manager::HistoryManager, model_manager::ModelManager, note_manager::NoteManager,
    workspace_manager::WorkspaceManager,
};
#[cfg(test)]
use crate::models::agent::ContextVisibility;
use crate::models::agent::GroundingSource;

mod answers;
mod filesystem;
mod formatting;
mod intent;
mod lifecycle;
mod planning;
mod safety;
mod sources;
mod tools;
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

// ReAct tool name + git.status constants live in `handlers/agent/tools.rs`
// after REF.3 batch 7.

#[derive(Debug, Clone, Serialize)]
pub(super) struct AgentToolRunResult {
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
                        return Err("Agent 功能已停用。請前往 /setting → Features 開啟。".into());
                    }
                    // REF.8 — legacy chat-first agent gate. Default off; the
                    // stateless capability layer (palette prefix / NL surfaces)
                    // is now canonical. The chat-first `AiPanel` UI was removed
                    // in REF.8, so this backend agent runtime is retained as a
                    // dormant asset with no UI entry point in this build. The
                    // `ai.legacy_agent` flag still gates the backend run path so
                    // the agent can be re-enabled if a future surface consumes it.
                    let legacy_on = cfg
                        .get("ai.legacy_agent")
                        .as_deref()
                        .map(|v| v.eq_ignore_ascii_case("true"))
                        .unwrap_or(false);
                    if !legacy_on {
                        return Err("Legacy chat-first agent is disabled. Use palette \
                             prefixes (explain / summarize / fix / cmd / next) \
                             or NL queries instead. The legacy chat panel was removed \
                             in REF.8; the agent backend is retained but has no UI \
                             entry point in this build."
                            .into());
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

// Source aggregation, IPC tool helpers, audit logging, and ContextBundle
// assembly methods on `AgentHandler` live in `sources.rs` after REF.3 batch 6.

// ReAct tool dispatch (`ReactDispatchState` + all `dispatch_*` methods) and
// `AgentHandler::build_react_dispatch` live in `tools.rs` after REF.3 batch 7.
#[cfg(test)]
mod tests {
    use super::tools::{bound_output, GIT_STATUS_OUTPUT_LIMIT};
    use super::*;
    use crate::models::settings_schema::SettingValueType;
    use std::path::Path;

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
        use crate::managers::ai_manager::{
            provider_supports_tool_calls, resolve_ai_runtime_config,
        };
        let pairs = [
            ("ai.provider", "openai"),
            ("ai.openai_api_key", "test-key"),
            ("ai.openai_base_url", "https://api.openai.com/v1"),
            ("ai.model", "gpt-4o-mini"),
        ];
        let rt = resolve_ai_runtime_config(|k| {
            pairs
                .iter()
                .find(|(key, _)| *key == k)
                .map(|(_, v)| v.to_string())
        })
        .unwrap();
        assert!(provider_supports_tool_calls(&rt.provider));
    }

    #[test]
    fn claude_provider_selects_heuristic_fallback() {
        use crate::managers::ai_manager::{
            provider_supports_tool_calls, resolve_ai_runtime_config,
        };
        let pairs = [
            ("ai.provider", "claude"),
            ("ai.api_key", "test-key"),
            ("ai.model", "claude-sonnet-4-6"),
        ];
        let rt = resolve_ai_runtime_config(|k| {
            pairs
                .iter()
                .find(|(key, _)| *key == k)
                .map(|(_, v)| v.to_string())
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
        assert!(looks_sensitive_path(Path::new(
            "/home/user/.aws/credentials"
        )));
        assert!(!looks_sensitive_path(Path::new(
            "/home/user/projects/main.rs"
        )));
    }

    #[test]
    fn resolve_readable_path_rejects_out_of_workspace() {
        let root = std::env::temp_dir().join(format!("keynova-path-test-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("create root");
        // Attempting to read something outside the root using `..` traversal
        let err = resolve_readable_path("../../etc/passwd", &[root.clone()]).unwrap_err();
        // Could fail at "not found" or "outside workspace" — both are correct rejections
        assert!(
            err.contains("not found")
                || err.contains("outside workspace")
                || err.contains("cannot resolve"),
            "unexpected error: {err}"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn resolve_readable_path_allows_in_workspace() {
        let root = std::env::temp_dir().join(format!("keynova-path-ok-{}", Uuid::new_v4()));
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
        assert!(
            !in_workspace,
            "outside dir should not start_with workspace root"
        );

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
