use serde_json::{json, Value};

use crate::models::workflow::WorkflowAction;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PipelineParseError {
    EmptyInput,
    EmptySegment { index: usize },
    UnknownCommand { index: usize, command: String },
}

impl std::fmt::Display for PipelineParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PipelineParseError::EmptyInput => write!(f, "pipeline input is empty"),
            PipelineParseError::EmptySegment { index } => {
                write!(f, "pipeline segment {index} is empty")
            }
            PipelineParseError::UnknownCommand { index, command } => {
                write!(
                    f,
                    "unknown command '{command}' in segment {index}; \
                     supported: search, ai, agent, note, open, history"
                )
            }
        }
    }
}

/// Parse `|`-separated pipeline text into a sequence of `WorkflowAction`s.
///
/// Each segment is trimmed; a leading `/` on the first token is stripped before
/// command lookup. Nested `|` characters (inside quotes) are NOT supported in v0.
pub fn parse_pipeline_text(text: &str) -> Result<Vec<WorkflowAction>, PipelineParseError> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(PipelineParseError::EmptyInput);
    }

    let segments: Vec<&str> = trimmed.split('|').collect();
    let mut actions = Vec::with_capacity(segments.len());

    for (index, segment) in segments.iter().enumerate() {
        let segment = segment.trim();
        if segment.is_empty() {
            return Err(PipelineParseError::EmptySegment { index });
        }

        let action = parse_segment(index, segment)?;
        actions.push(action);
    }

    Ok(actions)
}

fn parse_segment(index: usize, segment: &str) -> Result<WorkflowAction, PipelineParseError> {
    // Split into command token and the rest as args.
    let (raw_cmd, args) = segment
        .split_once(char::is_whitespace)
        .map(|(cmd, rest)| (cmd, rest.trim()))
        .unwrap_or((segment, ""));

    // Strip leading `/` from command token (e.g., `/search` → `search`).
    let cmd = raw_cmd.strip_prefix('/').unwrap_or(raw_cmd);

    map_command_to_action(index, cmd, args)
}

fn map_command_to_action(
    index: usize,
    cmd: &str,
    args: &str,
) -> Result<WorkflowAction, PipelineParseError> {
    match cmd {
        "search" => Ok(WorkflowAction {
            route: "search.query".into(),
            payload: json!({ "query": args }),
        }),
        "ai" | "agent" => Ok(WorkflowAction {
            route: "agent.start".into(),
            payload: json!({ "prompt": args }),
        }),
        "note" => Ok(WorkflowAction {
            route: "cmd.run".into(),
            payload: json!({ "name": "note", "args": args }),
        }),
        "open" => Ok(WorkflowAction {
            route: "launcher.launch".into(),
            payload: json!({ "path": args }),
        }),
        "history" => Ok(WorkflowAction {
            route: "history.search".into(),
            payload: json!({ "query": args }),
        }),
        other => Err(PipelineParseError::UnknownCommand {
            index,
            command: other.to_string(),
        }),
    }
}

/// Return the canonical set of supported pipeline command tokens.
pub fn supported_commands() -> &'static [&'static str] {
    &["search", "ai", "agent", "note", "open", "history"]
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── P1.A: parse_pipeline_text ───────────────────────────────────────────

    #[test]
    fn parse_empty_input_returns_error() {
        assert_eq!(
            parse_pipeline_text("   "),
            Err(PipelineParseError::EmptyInput)
        );
    }

    #[test]
    fn parse_empty_segment_returns_error() {
        let err = parse_pipeline_text("search foo | | ai explain").unwrap_err();
        assert_eq!(err, PipelineParseError::EmptySegment { index: 1 });
    }

    #[test]
    fn parse_unknown_command_returns_readable_error() {
        let err = parse_pipeline_text("grep foo").unwrap_err();
        assert!(matches!(err, PipelineParseError::UnknownCommand { index: 0, .. }));
        assert!(err.to_string().contains("grep"));
        assert!(err.to_string().contains("supported:"));
    }

    #[test]
    fn parse_search_then_ai_pipeline() {
        let actions = parse_pipeline_text("/search foo | ai explain").unwrap();
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0].route, "search.query");
        assert_eq!(actions[0].payload["query"], "foo");
        assert_eq!(actions[1].route, "agent.start");
        assert_eq!(actions[1].payload["prompt"], "explain");
    }

    #[test]
    fn parse_single_command_without_pipe() {
        let actions = parse_pipeline_text("search keynova").unwrap();
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].route, "search.query");
        assert_eq!(actions[0].payload["query"], "keynova");
    }

    #[test]
    fn parse_agent_alias_maps_to_agent_start() {
        let actions = parse_pipeline_text("agent summarize this").unwrap();
        assert_eq!(actions[0].route, "agent.start");
        assert_eq!(actions[0].payload["prompt"], "summarize this");
    }

    #[test]
    fn parse_open_command_maps_path() {
        let actions = parse_pipeline_text("open /home/user/doc.md").unwrap();
        assert_eq!(actions[0].route, "launcher.launch");
        assert_eq!(actions[0].payload["path"], "/home/user/doc.md");
    }

    #[test]
    fn parse_history_command_maps_query() {
        let actions = parse_pipeline_text("history rust").unwrap();
        assert_eq!(actions[0].route, "history.search");
        assert_eq!(actions[0].payload["query"], "rust");
    }

    #[test]
    fn parse_note_command_maps_args() {
        let actions = parse_pipeline_text("note today's standup").unwrap();
        assert_eq!(actions[0].route, "cmd.run");
        assert_eq!(actions[0].payload["name"], "note");
        assert_eq!(actions[0].payload["args"], "today's standup");
    }

    #[test]
    fn parse_slash_prefix_stripped_from_first_segment() {
        let with_slash = parse_pipeline_text("/search foo").unwrap();
        let without_slash = parse_pipeline_text("search foo").unwrap();
        assert_eq!(with_slash[0].route, without_slash[0].route);
        assert_eq!(with_slash[0].payload, without_slash[0].payload);
    }

    #[test]
    fn parse_extra_whitespace_is_trimmed() {
        let actions = parse_pipeline_text("  search   rust async  |  ai   explain  ").unwrap();
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0].payload["query"], "rust async");
        assert_eq!(actions[1].payload["prompt"], "explain");
    }
}