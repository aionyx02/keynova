//! Capability: generate a single shell command for a natural-language intent.
//!
//! Returns a structured payload so REF.6.F can render a dedicated command card.
//! The capability does not execute the command; it only suggests one. Risk is
//! still surfaced conservatively so UI-owned confirmation can gate future
//! direct-run affordances.

use std::sync::atomic::Ordering;

use serde::{Deserialize, Serialize};

use crate::core::ai_capability::command::risk_tag_for_command;
use crate::core::ai_capability::contract::{
    CapabilityDeps, CapabilityError, CapabilityOutput, CapabilityRequest, CapabilityResponse,
    CapabilitySource,
};
use crate::core::ai_capability::memory::push_memory_sources;
use crate::core::ai_capability::parse::extract_first_json_object;
use crate::core::ai_capability::prompt::{build_prompt_with_sources, maybe_audit};
use crate::core::ai_capability::registry::{meta, CapabilityId};
use crate::models::agent::GroundingSource;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct GenCommandCtx {
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub shell: Option<String>,
    #[serde(default)]
    pub os: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Payload {
    intent: String,
    #[serde(default)]
    ctx: GenCommandCtx,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GenCommandOutput {
    pub command: String,
    pub confidence: f32,
    pub rationale: String,
    #[serde(default)]
    pub assumptions: GenCommandCtx,
}

const SYSTEM: &str = "You are a command-generation assistant inside Keynova, a keyboard-first \
     launcher. Return exactly one shell command that best matches the user's intent. Reply with \
     strict JSON only: {\"command\": string, \"confidence\": number, \"rationale\": string}. \
     Confidence must be a number between 0.0 and 1.0. Never wrap the JSON in markdown fences.";

pub fn call(
    req: CapabilityRequest,
    deps: &CapabilityDeps,
) -> Result<CapabilityResponse, CapabilityError> {
    let payload: Payload = serde_json::from_value(req.payload)
        .map_err(|e| CapabilityError::InvalidPayload(e.to_string()))?;
    let intent = payload.intent.trim();
    if intent.is_empty() {
        return Err(CapabilityError::InvalidPayload("intent is empty".into()));
    }
    if deps.cancel.load(Ordering::SeqCst) {
        return Err(CapabilityError::Cancelled);
    }

    let mut sources: Vec<GroundingSource> = Vec::new();
    if let Some(lc) = deps.local_context.as_ref() {
        lc.push_workspace_source(&mut sources);
        let q = intent.to_lowercase();
        let _ = lc.push_command_sources(&q, &mut sources);
        let _ = lc.push_history_sources(&q, &mut sources);
        lc.push_model_sources(&q, &mut sources);
    }
    // Personal-memory grounding is gated to local providers (ADR-0043).
    if deps.allow_memory_grounding {
        if let Some(store) = deps.knowledge_store.as_ref() {
            push_memory_sources(store, intent, &mut sources);
        }
    }

    let assumptions = resolve_ctx(payload.ctx, deps.local_context.as_ref());
    let mut task = format!("Intent: {intent}\nReturn the best single command for this intent.");
    if let Some(cwd) = assumptions.cwd.as_deref() {
        task.push_str(&format!("\nCurrent working directory: {cwd}"));
    }
    if let Some(shell) = assumptions.shell.as_deref() {
        task.push_str(&format!("\nShell: {shell}"));
    }
    if let Some(os) = assumptions.os.as_deref() {
        task.push_str(&format!("\nOperating system: {os}"));
    }
    task.push_str(
        "\nPrefer built-in safe inspection commands when the intent is ambiguous. \
         Avoid destructive commands unless the intent explicitly asks for them.",
    );

    let audit = meta(CapabilityId::GenCommand).audit;
    let built_prompt = build_prompt_with_sources(SYSTEM, &sources, &task);
    match deps.chat.chat(&built_prompt.text, &deps.cancel) {
        Ok(reply) => {
            let mut output = parse_output(&reply);
            output.assumptions = assumptions;
            maybe_audit(
                deps.knowledge_store.as_ref(),
                audit,
                CapabilityId::GenCommand.as_str(),
                "ok",
                "capability:gen_command completed",
                None,
            );
            Ok(CapabilityResponse {
                id: CapabilityId::GenCommand,
                risk_tag: risk_tag_for_command(&output.command),
                output: CapabilityOutput::Structured {
                    value: serde_json::to_value(output)
                        .expect("GenCommandOutput must serialize to JSON"),
                },
                sources: CapabilitySource::from_grounding_sources(&built_prompt.included_sources),
            })
        }
        Err(error) => {
            maybe_audit(
                deps.knowledge_store.as_ref(),
                audit,
                CapabilityId::GenCommand.as_str(),
                "error",
                &error,
                None,
            );
            Err(CapabilityError::ProviderError(error))
        }
    }
}

fn parse_output(reply: &str) -> GenCommandOutput {
    if let Ok(parsed) = serde_json::from_str::<GenCommandOutput>(reply.trim()) {
        return normalize_output(parsed);
    }
    if let Some(json) = extract_first_json_object(reply) {
        if let Ok(parsed) = serde_json::from_str::<GenCommandOutput>(json) {
            return normalize_output(parsed);
        }
    }

    let command = fallback_command(reply);
    let rationale = if command.is_empty() {
        "Model did not return a usable command.".to_string()
    } else {
        "Model returned non-JSON output; using the first command-shaped line.".to_string()
    };
    normalize_output(GenCommandOutput {
        command,
        confidence: 0.25,
        rationale,
        assumptions: GenCommandCtx::default(),
    })
}

fn normalize_output(mut parsed: GenCommandOutput) -> GenCommandOutput {
    parsed.command = parsed.command.trim().trim_matches('`').to_string();
    parsed.rationale = parsed.rationale.trim().to_string();
    parsed.confidence = parsed.confidence.clamp(0.0, 1.0);
    parsed
}

fn resolve_ctx(
    ctx: GenCommandCtx,
    local_context: Option<&crate::core::local_context::LocalContextSearcher>,
) -> GenCommandCtx {
    fn normalize(value: Option<String>) -> Option<String> {
        value
            .map(|item| item.trim().to_string())
            .filter(|item| !item.is_empty())
    }

    let workspace_cwd = local_context.and_then(|local_context| {
        local_context
            .workspace_manager
            .lock()
            .ok()
            .and_then(|workspace| workspace.current().project_root.clone())
    });
    let process_cwd = std::env::current_dir()
        .ok()
        .map(|path| path.to_string_lossy().into_owned());

    GenCommandCtx {
        cwd: normalize(ctx.cwd).or(workspace_cwd).or(process_cwd),
        shell: normalize(ctx.shell).or_else(runtime_shell),
        os: normalize(ctx.os).or_else(|| Some(std::env::consts::OS.to_string())),
    }
}

fn runtime_shell() -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        std::env::var("COMSPEC")
            .ok()
            .or_else(|| Some("cmd.exe".into()))
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("SHELL")
            .ok()
            .or_else(|| Some("/bin/sh".into()))
    }
}

fn fallback_command(reply: &str) -> String {
    for line in reply.lines() {
        let trimmed = line.trim().trim_matches('`');
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with('{') || trimmed.starts_with("```") {
            continue;
        }
        if trimmed.ends_with(':') {
            continue;
        }
        return trimmed.to_string();
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ai_capability::contract::ChatProvider;
    use crate::core::ai_capability::test_fixtures::{
        assert_invalid_payload, assert_safe_primary_output, COMMAND_REQUEST,
    };
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    struct JsonProvider;
    impl ChatProvider for JsonProvider {
        fn chat(&self, _prompt: &str, _cancel: &AtomicBool) -> Result<String, String> {
            Ok(
                r#"{"command":"git status","confidence":0.9,"rationale":"Inspect the branch before taking action."}"#
                    .into(),
            )
        }
    }

    struct FallbackProvider;
    impl ChatProvider for FallbackProvider {
        fn chat(&self, _prompt: &str, _cancel: &AtomicBool) -> Result<String, String> {
            Ok("Use this:\n`cargo test -p keynova`".into())
        }
    }

    fn deps(chat: Arc<dyn ChatProvider>) -> CapabilityDeps {
        CapabilityDeps {
            chat,
            local_context: None,
            knowledge_store: None,
            cancel: Arc::new(AtomicBool::new(false)),
            stream_chunk: None,
            allow_memory_grounding: false,
        }
    }

    fn req(payload: serde_json::Value) -> CapabilityRequest {
        CapabilityRequest {
            id: CapabilityId::GenCommand,
            payload,
            context_hash: None,
        }
    }

    #[test]
    fn rejects_empty_intent() {
        let err = call(
            req(serde_json::json!({ "intent": "   " })),
            &deps(Arc::new(JsonProvider)),
        )
        .unwrap_err();
        assert_invalid_payload(err);
    }

    #[test]
    fn rejects_malformed_payload_as_typed_error() {
        let err = call(
            req(serde_json::json!({ "intent": { "nested": true } })),
            &deps(Arc::new(JsonProvider)),
        )
        .unwrap_err();
        assert_invalid_payload(err);
    }

    #[test]
    fn happy_path_returns_structured_output() {
        let resp = call(
            req(serde_json::json!({
                "intent": COMMAND_REQUEST,
                "ctx": {
                    "cwd": " C:/work/keynova ",
                    "shell": " powershell ",
                    "os": " windows "
                }
            })),
            &deps(Arc::new(JsonProvider)),
        )
        .unwrap();
        assert_eq!(resp.id, CapabilityId::GenCommand);
        assert!(!resp.risk_tag.requires_confirmation);
        assert_safe_primary_output(&resp);
        match resp.output {
            CapabilityOutput::Structured { ref value } => {
                let parsed: GenCommandOutput = serde_json::from_value(value.clone()).unwrap();
                assert_eq!(parsed.command, "git status");
                assert_eq!(parsed.confidence, 0.9);
                assert_eq!(parsed.assumptions.cwd.as_deref(), Some("C:/work/keynova"));
                assert_eq!(parsed.assumptions.shell.as_deref(), Some("powershell"));
                assert_eq!(parsed.assumptions.os.as_deref(), Some("windows"));
            }
            _ => panic!("expected structured output"),
        }
    }

    #[test]
    fn falls_back_when_model_returns_non_json() {
        let resp = call(
            req(serde_json::json!({ "intent": "run tests", "ctx": {} })),
            &deps(Arc::new(FallbackProvider)),
        )
        .unwrap();
        match resp.output {
            CapabilityOutput::Structured { ref value } => {
                let parsed: GenCommandOutput = serde_json::from_value(value.clone()).unwrap();
                assert_eq!(parsed.command, "cargo test -p keynova");
                assert!(parsed.confidence <= 0.25);
            }
            _ => panic!("expected structured output"),
        }
        assert!(!resp.risk_tag.requires_confirmation);
        assert_safe_primary_output(&resp);
    }

    #[test]
    fn malformed_model_json_returns_safe_structured_fallback() {
        let parsed = parse_output("{not valid json");
        assert!(parsed.command.is_empty());
        assert_eq!(parsed.confidence, 0.25);
        assert!(parsed.rationale.contains("usable command"));
    }

    #[test]
    fn fills_runtime_assumptions_when_context_is_omitted() {
        let resp = call(
            req(serde_json::json!({ "intent": COMMAND_REQUEST })),
            &deps(Arc::new(JsonProvider)),
        )
        .unwrap();
        match resp.output {
            CapabilityOutput::Structured { value } => {
                let parsed: GenCommandOutput = serde_json::from_value(value).unwrap();
                assert!(parsed.assumptions.cwd.is_some());
                assert!(parsed.assumptions.shell.is_some());
                assert!(parsed.assumptions.os.is_some());
            }
            _ => panic!("expected structured output"),
        }
    }

    #[test]
    fn extracts_json_inside_markdown_fence() {
        let reply = "```json\n{\"command\":\"git diff\",\"confidence\":0.6,\"rationale\":\"Review changes first.\"}\n```";
        let parsed = parse_output(reply);
        assert_eq!(parsed.command, "git diff");
        assert_eq!(parsed.confidence, 0.6);
    }

    #[test]
    fn risk_tag_confirms_non_read_only_commands() {
        let tag = risk_tag_for_command("git push origin HEAD");
        assert!(tag.requires_confirmation);
    }
}
