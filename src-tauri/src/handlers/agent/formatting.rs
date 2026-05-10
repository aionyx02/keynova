use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use crate::core::config_manager::ConfigManager;
use crate::models::agent::{
    AgentFilteredSource, AgentPlannedAction, AgentPromptAudit, ContextVisibility, GroundingSource,
};
use crate::models::builtin_command::{BuiltinCommandResult, CommandUiType};
use crate::models::settings_schema::{builtin_setting_schema, SettingValueType};
use crate::models::terminal::TerminalLaunchSpec;

use uuid::Uuid;

use super::intent::should_run_local_search;
use super::safety::contains_any;
use super::PROMPT_SOURCE_LIMIT;

pub(super) fn build_prompt_audit(
    prompt: &str,
    sources: &[GroundingSource],
    budget_chars: usize,
) -> AgentPromptAudit {
    let mut remaining = budget_chars.saturating_sub(prompt.chars().count());
    let mut included_sources = Vec::new();
    let mut filtered_sources = Vec::new();
    let mut truncated = false;
    let mut redacted_secret_count = 0usize;

    for source in sources {
        match source.visibility {
            ContextVisibility::PublicContext | ContextVisibility::UserPrivate => {
                let weight = source.title.chars().count() + source.snippet.chars().count() + 24;
                if included_sources.len() >= PROMPT_SOURCE_LIMIT || weight > remaining {
                    truncated = true;
                    continue;
                }
                remaining = remaining.saturating_sub(weight);
                included_sources.push(source.clone());
            }
            ContextVisibility::PrivateArchitecture | ContextVisibility::Secret => {
                if source.visibility == ContextVisibility::Secret {
                    redacted_secret_count += 1;
                }
                filtered_sources.push(AgentFilteredSource {
                    source_id: source.source_id.clone(),
                    source_type: source.source_type.clone(),
                    title: source.title.clone(),
                    visibility: source.visibility,
                    reason: source
                        .redacted_reason
                        .clone()
                        .unwrap_or_else(|| "filtered".into()),
                });
            }
        }
    }

    let prompt_chars = prompt.chars().count()
        + included_sources
            .iter()
            .map(|source| source.title.chars().count() + source.snippet.chars().count())
            .sum::<usize>();

    AgentPromptAudit {
        budget_chars,
        prompt_chars,
        truncated,
        included_sources,
        filtered_sources,
        redacted_secret_count,
    }
}


pub(super) fn capability_answer() -> String {
    [
        "我可以用兩種方式幫你：",
        "",
        "1. 直接回答本機可判斷的小問題，例如目前時間、Keynova 能力、下一步建議。",
        "2. 先規劃再等你批准本機動作，例如開 panel、建立筆記草稿、修改設定草稿、執行安全內建命令，或準備 terminal/file/system/model 這類高風險動作。",
        "3. 用 read-only tools 搜尋 Keynova context，並把 private architecture / secrets 留在本機 audit，不送進外部查詢。",
        "",
        "目前還沒完全像一般 agent 的原因是：Agent mode 尚未把 `ai.chat` provider 接回 `agent.run`，所以一般開放式聊天仍比較適合 Chat mode。下一步要做的是 Ask/Act 合一：一般問題走 AI provider，需要本機動作時才切 approval。",
    ]
    .join("\n")
}

pub(super) fn build_plan(
    prompt: &str,
    action: Option<&AgentPlannedAction>,
    has_direct_answer: bool,
) -> Vec<String> {
    let mut plan = vec![
        "Classify requested context by visibility before building any prompt".into(),
        "Use read-only tools first and keep private architecture and secrets out of model context"
            .into(),
    ];
    if let Some(action) = action {
        plan.push(format!(
            "Prepare '{}' as a {:?} action and wait for explicit approval",
            action.label, action.risk
        ));
    } else if has_direct_answer {
        plan.push("Answer directly from Keynova's local agent runtime".into());
    } else if should_run_local_search(prompt) {
        plan.push("Return a grounded plan using local Keynova context only".into());
    } else {
        plan.push("Explain how to ask for chat, search, or approved local actions".into());
    }
    plan
}

pub(super) fn describe_run(prompt: &str, audit: &AgentPromptAudit) -> String {
    if should_run_local_search(prompt) && !audit.included_sources.is_empty() {
        return format!(
            "我找到 {} 個可用的 Keynova 參考來源。你可以展開 Context audit 看細節，或直接要求我「開啟設定」、「建立筆記草稿」、「搜尋 note/history/model」這類可批准動作。",
            audit.included_sources.len()
        );
    }
    if audit.filtered_sources.is_empty() {
        "我目前沒有偵測到需要執行的本機動作。你可以直接問「你可以做什麼」、「現在幾點」，或用更明確的動作語氣，例如「建立一篇筆記草稿」、「開啟設定」、「執行 `npm run build`」。一般聊天式回答會在下一步接上 AI provider 後變得更像標準 agent。".into()
    } else {
        format!(
            "我已完成安全檢查，但這次沒有可執行動作。{} 個來源因為 private/secret 規則被保留在本機 audit 中，沒有放進可用上下文。",
            audit.filtered_sources.len()
        )
    }
}

pub(super) fn describe_execution(action: &AgentPlannedAction, result: &BuiltinCommandResult) -> String {
    match &result.ui_type {
        CommandUiType::Inline => format!("Executed '{}'. {}", action.label, result.text),
        CommandUiType::Panel(panel) => {
            format!("Executed '{}'. Opened panel '{}'.", action.label, panel)
        }
        CommandUiType::Terminal(spec) => format!(
            "Executed '{}'. Ready to run terminal command '{}'.",
            action.label, spec.program
        ),
    }
}

pub(super) fn visibility_filtered_source(
    source_id: String,
    source_type: &str,
    title: String,
    snippet: String,
    score: f32,
) -> GroundingSource {
    let combined = format!("{title} {snippet}").to_lowercase();
    if contains_any(
        &combined,
        &[
            "CLAUDE.md",
            "tasks.md",
            "memory.md",
            "decisions.md",
            "skill.md",
            "private_architecture",
            "architecture",
        ],
    ) {
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
    if contains_any(
        &combined,
        &[
            "api_key", "api key", "password", "token", "secret", "sk-", "bearer ",
        ],
    ) {
        return GroundingSource {
            source_id,
            source_type: source_type.into(),
            title,
            snippet: "[redacted secret]".into(),
            uri: None,
            score,
            visibility: ContextVisibility::Secret,
            redacted_reason: Some("secret".into()),
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

pub(super) fn source(
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

pub(super) fn parse_visibility(value: &str) -> ContextVisibility {
    match value {
        "public_context" => ContextVisibility::PublicContext,
        "private_architecture" => ContextVisibility::PrivateArchitecture,
        "secret" => ContextVisibility::Secret,
        _ => ContextVisibility::UserPrivate,
    }
}

pub(super) fn suggested_note_name(prompt: &str) -> String {
    if let Some(quoted) = extract_quoted(prompt) {
        return quoted.replace(' ', "_");
    }
    let words = prompt
        .split_whitespace()
        .filter(|word| word.chars().any(|ch| ch.is_alphanumeric()))
        .take(4)
        .collect::<Vec<_>>();
    if words.is_empty() {
        "agent_draft".into()
    } else {
        words.join("_").to_lowercase()
    }
}

pub(super) fn title_case(value: &str) -> String {
    value
        .split(['_', '-', ' '])
        .filter(|segment| !segment.is_empty())
        .map(|segment| {
            let mut chars = segment.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn match_setting_schema(prompt: &str) -> Option<crate::models::settings_schema::SettingSchema> {
    let q = prompt.to_lowercase();
    builtin_setting_schema()
        .into_iter()
        .filter(|schema| !schema.sensitive)
        .map(|schema| {
            let mut score = 0i32;
            if q.contains(schema.key) {
                score += 4;
            }
            for token in schema.key.split('.') {
                if token.len() >= 3 && q.contains(token) {
                    score += 1;
                }
            }
            for token in schema.label.to_lowercase().split_whitespace() {
                if token.len() >= 3 && q.contains(token) {
                    score += 1;
                }
            }
            (score, schema)
        })
        .filter(|(score, _)| *score >= 2)
        .max_by_key(|(score, _)| *score)
        .map(|(_, schema)| schema)
}

pub(super) fn extract_setting_value(prompt: &str, value_type: &SettingValueType) -> Option<String> {
    let lower = prompt.to_lowercase();
    match value_type {
        SettingValueType::Boolean => {
            if contains_any(&lower, &["true", "enable", "enabled", "on", "開", "开启"]) {
                Some("true".into())
            } else if contains_any(
                &lower,
                &["false", "disable", "disabled", "off", "關", "关闭"],
            ) {
                Some("false".into())
            } else {
                None
            }
        }
        SettingValueType::Integer => prompt
            .split(|ch: char| !ch.is_ascii_digit())
            .rfind(|part| !part.is_empty())
            .map(ToOwned::to_owned),
        _ => extract_quoted(prompt).or_else(|| {
            prompt
                .split_whitespace()
                .last()
                .map(|value| value.trim_matches(|ch| ch == '.' || ch == ',').to_string())
        }),
    }
}

pub(super) fn extract_shell_command(prompt: &str) -> Option<String> {
    let lower = prompt.to_lowercase();
    if !contains_any(
        &lower,
        &[
            "run ", "execute ", "start ", "terminal", "cargo ", "npm ", "git ", "pnpm ", "python ",
            "執行", "运行",
        ],
    ) {
        return None;
    }
    if let Some(quoted) = extract_backticked(prompt) {
        return Some(quoted);
    }
    for marker in ["run ", "execute ", "start ", "執行", "运行"] {
        if let Some(index) = lower.find(marker) {
            let value = prompt[index + marker.len()..].trim();
            if looks_like_shell_command(value) {
                return Some(value.to_string());
            }
        }
    }
    for marker in [
        "cargo ", "npm ", "git ", "pnpm ", "python ", "python3 ", "node ", "npx ", "yarn ", "bun ",
    ] {
        if let Some(index) = lower.find(marker) {
            let value = prompt[index..].trim();
            if looks_like_shell_command(value) {
                return Some(value.to_string());
            }
        }
    }
    None
}

pub(super) fn looks_like_shell_command(value: &str) -> bool {
    let command = value
        .trim()
        .trim_matches(|ch| matches!(ch, '"' | '\'' | '`' | ':' | ',' | '.'));
    if command.is_empty() {
        return false;
    }
    let Some(program) = command.split_whitespace().next() else {
        return false;
    };
    let program = program
        .trim_matches(|ch| matches!(ch, '"' | '\'' | '`'))
        .to_lowercase();

    if program.starts_with("./")
        || program.starts_with(".\\")
        || program.contains('\\')
        || program.ends_with(".exe")
        || program.ends_with(".cmd")
        || program.ends_with(".bat")
        || program.ends_with(".ps1")
        || program.ends_with(".sh")
    {
        return true;
    }

    matches!(
        program.as_str(),
        "cargo"
            | "npm"
            | "pnpm"
            | "yarn"
            | "bun"
            | "npx"
            | "node"
            | "git"
            | "python"
            | "python3"
            | "py"
            | "rustup"
            | "tauri"
            | "powershell"
            | "powershell.exe"
            | "pwsh"
            | "pwsh.exe"
            | "cmd"
            | "cmd.exe"
            | "bash"
            | "sh"
            | "ls"
            | "dir"
            | "cat"
            | "type"
            | "mkdir"
            | "rm"
            | "del"
            | "copy"
            | "move"
            | "echo"
    )
}

pub(super) fn extract_backticked(prompt: &str) -> Option<String> {
    let start = prompt.find('`')?;
    let rest = &prompt[start + 1..];
    let end = rest.find('`')?;
    let value = rest[..end].trim();
    (!value.is_empty()).then(|| value.to_string())
}

pub(super) fn extract_quoted(prompt: &str) -> Option<String> {
    ['"', '\''].into_iter().find_map(|quote| {
        let start = prompt.find(quote)?;
        let rest = &prompt[start + quote.len_utf8()..];
        let end = rest.find(quote)?;
        let value = rest[..end].trim();
        (!value.is_empty()).then(|| value.to_string())
    })
}

pub(super) fn extract_path_like(prompt: &str) -> Option<String> {
    prompt
        .split_whitespace()
        .map(|token| token.trim_matches(|ch| matches!(ch, '"' | '\'' | ',' | '.')))
        .find(|token| {
            token.contains('\\')
                || token.contains('/')
                || token.ends_with(".md")
                || token.ends_with(".txt")
                || token.ends_with(".json")
                || token.ends_with(".rs")
                || token.ends_with(".ts")
        })
        .map(ToOwned::to_owned)
}

pub(super) fn build_terminal_command_spec(
    config: &Arc<Mutex<ConfigManager>>,
    command: &str,
) -> TerminalLaunchSpec {
    let configured_shell = config
        .lock()
        .ok()
        .and_then(|cfg| cfg.get("terminal.default_shell"))
        .filter(|value| !value.trim().is_empty());

    #[cfg(target_os = "windows")]
    let (program, args) = {
        let program = configured_shell.unwrap_or_else(|| "powershell.exe".into());
        let lower = program.to_lowercase();
        let args = if lower.ends_with("cmd.exe") || lower == "cmd.exe" {
            vec!["/C".into(), command.to_string()]
        } else if lower.ends_with("pwsh.exe")
            || lower.ends_with("powershell.exe")
            || lower == "pwsh.exe"
            || lower == "powershell.exe"
        {
            vec!["-NoLogo".into(), "-Command".into(), command.to_string()]
        } else {
            vec!["-c".into(), command.to_string()]
        };
        (program, args)
    };

    #[cfg(not(target_os = "windows"))]
    let (program, args) = {
        let program = configured_shell
            .or_else(|| std::env::var("SHELL").ok())
            .unwrap_or_else(|| "/bin/sh".into());
        (program, vec!["-lc".into(), command.to_string()])
    };

    TerminalLaunchSpec {
        launch_id: Uuid::new_v4().to_string(),
        program,
        args,
        cwd: std::env::current_dir()
            .ok()
            .map(|path| path.display().to_string()),
        title: Some(format!("Agent: {command}")),
        env: Vec::new(),
        editor: false,
    }
}


pub(super) fn truncate(value: &str, max_chars: usize) -> String {
    let mut out = String::new();
    let mut truncated = false;
    for (count, ch) in value.chars().enumerate() {
        if count >= max_chars {
            truncated = true;
            break;
        }
        out.push(ch);
    }
    if truncated {
        out.push_str("...");
    }
    out
}


pub(super) fn inline_result(text: String) -> BuiltinCommandResult {
    BuiltinCommandResult {
        text,
        ui_type: CommandUiType::Inline,
    }
}

pub(super) fn panel_result(name: &str, initial_args: String) -> BuiltinCommandResult {
    BuiltinCommandResult {
        text: initial_args,
        ui_type: CommandUiType::Panel(name.to_string()),
    }
}

pub(super) fn require_str<'a>(payload: &'a Value, key: &str) -> Result<&'a str, String> {
    payload
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("missing or empty '{key}'"))
}

pub(super) fn grounding_to_tool_sources_json(sources: &[GroundingSource]) -> Value {
    json!({
        "sources": sources.iter().map(|s| json!({
            "title": s.title,
            "snippet": s.snippet,
            "uri": s.uri,
            "source_type": s.source_type,
        })).collect::<Vec<_>>()
    })
}
