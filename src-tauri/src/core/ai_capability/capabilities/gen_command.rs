//! Capability: generate a single shell command for a natural-language intent.
//!
//! Returns a structured payload so REF.6.F can render a dedicated command card.
//! The capability does not execute the command; it only suggests one. Risk is
//! still surfaced conservatively so UI-owned confirmation can gate future
//! direct-run affordances.

use std::sync::atomic::Ordering;

use serde::{Deserialize, Serialize};

use crate::core::ai_capability::contract::{
    CapabilityDeps, CapabilityError, CapabilityOutput, CapabilityRequest, CapabilityResponse,
};
use crate::core::ai_capability::parse::extract_first_json_object;
use crate::core::ai_capability::prompt::{build_prompt, maybe_audit};
use crate::core::ai_capability::registry::{meta, CapabilityId};
use crate::models::agent::GroundingSource;
use crate::models::unified_result::RiskTag;

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

    let mut task = format!("Intent: {intent}\nReturn the best single command for this intent.");
    if let Some(cwd) = payload.ctx.cwd.as_deref().filter(|v| !v.trim().is_empty()) {
        task.push_str(&format!("\nCurrent working directory: {cwd}"));
    }
    if let Some(shell) = payload
        .ctx
        .shell
        .as_deref()
        .filter(|v| !v.trim().is_empty())
    {
        task.push_str(&format!("\nShell: {shell}"));
    }
    if let Some(os) = payload.ctx.os.as_deref().filter(|v| !v.trim().is_empty()) {
        task.push_str(&format!("\nOperating system: {os}"));
    }
    task.push_str(
        "\nPrefer built-in safe inspection commands when the intent is ambiguous. \
         Avoid destructive commands unless the intent explicitly asks for them.",
    );

    let audit = meta(CapabilityId::GenCommand).audit;
    let prompt = build_prompt(SYSTEM, &sources, &task);
    match deps.chat.chat(&prompt, &deps.cancel) {
        Ok(reply) => {
            let output = parse_output(&reply);
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
    })
}

fn normalize_output(mut parsed: GenCommandOutput) -> GenCommandOutput {
    parsed.command = parsed.command.trim().trim_matches('`').to_string();
    parsed.rationale = parsed.rationale.trim().to_string();
    parsed.confidence = parsed.confidence.clamp(0.0, 1.0);
    parsed
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

fn risk_tag_for_command(command: &str) -> RiskTag {
    let normalized = command.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return RiskTag::confirm("generated command was empty");
    }

    const SAFE_PREFIXES: &[&str] = &[
        "rg ",
        "grep ",
        "ls",
        "dir",
        "cat ",
        "type ",
        "git status",
        "git diff",
        "git log",
        "cargo test",
        "cargo check",
        "cargo clippy",
        "npm test",
        "npm run lint",
        "pnpm test",
        "yarn test",
    ];

    if SAFE_PREFIXES
        .iter()
        .any(|prefix| normalized.starts_with(prefix))
    {
        RiskTag::none()
    } else {
        RiskTag::confirm("generated command may change local system state")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ai_capability::contract::ChatProvider;
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
        assert!(matches!(err, CapabilityError::InvalidPayload(_)));
    }

    #[test]
    fn happy_path_returns_structured_output() {
        let resp = call(
            req(serde_json::json!({ "intent": "show git branch status", "ctx": {} })),
            &deps(Arc::new(JsonProvider)),
        )
        .unwrap();
        assert_eq!(resp.id, CapabilityId::GenCommand);
        assert!(!resp.risk_tag.requires_confirmation);
        match resp.output {
            CapabilityOutput::Structured { value } => {
                let parsed: GenCommandOutput = serde_json::from_value(value).unwrap();
                assert_eq!(parsed.command, "git status");
                assert_eq!(parsed.confidence, 0.9);
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
            CapabilityOutput::Structured { value } => {
                let parsed: GenCommandOutput = serde_json::from_value(value).unwrap();
                assert_eq!(parsed.command, "cargo test -p keynova");
                assert!(parsed.confidence <= 0.25);
            }
            _ => panic!("expected structured output"),
        }
        assert!(!resp.risk_tag.requires_confirmation);
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
