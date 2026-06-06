//! Capability: explain a compiler / linter error so the user can fix it.
//!
//! v1 is explanation-only per ADR-0029 §4 (initial three capabilities are
//! explanation-only). Payload variants that ask for file edits or shell
//! execution return `UnsupportedAction`. `audit = true`,
//! `requires_confirmation = false` (no destructive action).

use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::core::ai_capability::command::{risk_tag_for_command, CommandSuggestion};
use crate::core::ai_capability::contract::{
    CapabilityDeps, CapabilityError, CapabilityOutput, CapabilityRequest, CapabilityResponse,
    CapabilitySource,
};
use crate::core::ai_capability::memory::push_memory_sources;
use crate::core::ai_capability::prompt::{build_prompt_with_sources, maybe_audit};
use crate::core::ai_capability::registry::{meta, CapabilityId};
use crate::core::dev_runner::{
    extract_compiler_errors, run_bounded_dev_cmd, DEV_CARGO_TIMEOUT_SECS, DEV_NPM_TIMEOUT_SECS,
};
use crate::models::agent::GroundingSource;

/// Programs the read-only re-run path will accept. Anything else routes to
/// `UnsupportedAction` so a future destructive `fix_error` ADR can broaden
/// the surface explicitly.
const ALLOWED_PROGRAMS: &[&str] = &["cargo", "npm", "pnpm", "yarn", "tsc"];

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Payload {
    /// Re-run a known-safe dev command and use its captured output.
    RunCommand {
        program: String,
        args: Vec<String>,
        cwd: String,
        #[serde(default)]
        timeout_secs: Option<u64>,
    },
    /// Caller supplies raw compiler/linter output (preferred — no exec).
    RawOutput { raw_output: String },
    /// Any "apply" variant is rejected with UnsupportedAction in v1.
    Apply {
        #[serde(rename = "apply")]
        _apply: serde_json::Value,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FixErrorOutput {
    pub explanation: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggested_command: Option<CommandSuggestion>,
}

const SYSTEM: &str = "You are a senior developer's debugging assistant integrated into Keynova. \
     Given compiler, linter, or runtime output, turn it into a practical next step. \
     Use concise sections: Summary, Likely cause, Check, Next step, Optional command. \
     Keep commands copyable and non-destructive; never claim a command was run. \
     Be concrete about file paths and line numbers when present. \
     Never invent locations that are not in the input.";

pub fn call(
    req: CapabilityRequest,
    deps: &CapabilityDeps,
) -> Result<CapabilityResponse, CapabilityError> {
    let payload: Payload = serde_json::from_value(req.payload.clone())
        .map_err(|e| CapabilityError::InvalidPayload(e.to_string()))?;
    if deps.cancel.load(Ordering::SeqCst) {
        return Err(CapabilityError::Cancelled);
    }

    let raw_output = match payload {
        Payload::Apply { .. } => {
            return Err(CapabilityError::UnsupportedAction(
                "applying fixes is not supported in v1 — explanation-only".into(),
            ));
        }
        Payload::RawOutput { raw_output } => raw_output,
        Payload::RunCommand {
            program,
            args,
            cwd,
            timeout_secs,
        } => {
            if !ALLOWED_PROGRAMS.contains(&program.as_str()) {
                return Err(CapabilityError::UnsupportedAction(format!(
                    "program '{program}' is not in the bounded dev-runner allowlist"
                )));
            }
            let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
            let default_to = if program == "cargo" {
                DEV_CARGO_TIMEOUT_SECS
            } else {
                DEV_NPM_TIMEOUT_SECS
            };
            let timeout = Duration::from_secs(timeout_secs.unwrap_or(default_to));
            let value = run_bounded_dev_cmd(&program, &arg_refs, &PathBuf::from(cwd), timeout)
                .map_err(CapabilityError::ProviderError)?;
            let stdout = value
                .get("stdout")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let stderr = value
                .get("stderr")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            format!("{stdout}\n{stderr}")
        }
    };

    if raw_output.trim().is_empty() {
        return Err(CapabilityError::InvalidPayload(
            "no compiler/lint output to explain".into(),
        ));
    }
    if deps.cancel.load(Ordering::SeqCst) {
        return Err(CapabilityError::Cancelled);
    }

    let errors = extract_compiler_errors(&raw_output);
    let errors_block = if errors.is_empty() {
        format!("Raw output:\n{}", raw_output)
    } else {
        let mut s = String::from("Parsed errors:\n");
        for e in &errors {
            let location = e
                .get("location")
                .and_then(|v| v.as_str())
                .unwrap_or("(no location)");
            let code = e.get("code").and_then(|v| v.as_str()).unwrap_or("");
            let message = e.get("message").and_then(|v| v.as_str()).unwrap_or("");
            s.push_str(&format!("- [{code}] {message} @ {location}\n"));
        }
        s
    };

    let task = format!(
        "Explain the following compiler/lint error and suggest the next safe action. \
         If a command is useful, provide it as a copy-only suggestion and say why.\n\n{}",
        errors_block
    );

    // Personal-memory grounding is gated to local providers (ADR-0043).
    let mut sources: Vec<GroundingSource> = Vec::new();
    if deps.allow_memory_grounding {
        if let Some(store) = deps.knowledge_store.as_ref() {
            push_memory_sources(store, &raw_output, &mut sources);
        }
    }
    let built_prompt = build_prompt_with_sources(SYSTEM, &sources, &task);

    let reply = match &deps.stream_chunk {
        Some(on_chunk) => {
            deps.chat
                .chat_stream(&built_prompt.text, on_chunk.as_ref(), &deps.cancel)
        }
        None => deps.chat.chat(&built_prompt.text, &deps.cancel),
    };

    let audit = meta(CapabilityId::FixError).audit;
    match reply {
        Ok(text) if text.trim().is_empty() => {
            maybe_audit(
                deps.knowledge_store.as_ref(),
                audit,
                CapabilityId::FixError.as_str(),
                "error",
                "provider returned an empty response",
                None,
            );
            Err(CapabilityError::ProviderError(
                "provider returned an empty response".into(),
            ))
        }
        Ok(text) => {
            let output = parse_output(text);
            let risk_tag = output
                .suggested_command
                .as_ref()
                .map_or_else(crate::models::unified_result::RiskTag::none, |suggestion| {
                    risk_tag_for_command(&suggestion.command)
                });
            maybe_audit(
                deps.knowledge_store.as_ref(),
                audit,
                CapabilityId::FixError.as_str(),
                "ok",
                "capability:fix_error completed",
                None,
            );
            Ok(CapabilityResponse {
                id: CapabilityId::FixError,
                output: CapabilityOutput::Structured {
                    value: serde_json::to_value(output)
                        .expect("FixErrorOutput must serialize to JSON"),
                },
                risk_tag,
                sources: CapabilitySource::from_grounding_sources(&built_prompt.included_sources),
            })
        }
        Err(e) => {
            maybe_audit(
                deps.knowledge_store.as_ref(),
                audit,
                CapabilityId::FixError.as_str(),
                "error",
                &e,
                None,
            );
            Err(CapabilityError::ProviderError(e))
        }
    }
}

fn parse_output(explanation: String) -> FixErrorOutput {
    let suggested_command = extract_suggested_command(&explanation);
    let explanation = command_heading_index(&explanation)
        .map(|index| {
            explanation
                .lines()
                .take(index)
                .collect::<Vec<_>>()
                .join("\n")
                .trim()
                .to_string()
        })
        .filter(|text| !text.is_empty())
        .unwrap_or(explanation);
    FixErrorOutput {
        explanation,
        suggested_command,
    }
}

fn extract_suggested_command(explanation: &str) -> Option<CommandSuggestion> {
    let lines: Vec<&str> = explanation.lines().collect();
    let heading_index = command_heading_index(explanation)?;

    let heading = lines[heading_index].trim().trim_start_matches('#').trim();
    let inline = heading
        .split_once(':')
        .map(|(_, value)| value.trim())
        .filter(|value| !value.is_empty());

    let candidate = inline.or_else(|| {
        lines[heading_index + 1..]
            .iter()
            .map(|line| {
                line.trim()
                    .trim_start_matches(['-', '*'])
                    .trim()
                    .trim_matches('`')
                    .trim()
            })
            .find(|line| !line.is_empty() && *line != "```")
    })?;

    let normalized = candidate.trim_matches('`').trim();
    if normalized.is_empty()
        || matches!(
            normalized.to_ascii_lowercase().as_str(),
            "none" | "n/a" | "not needed"
        )
    {
        return None;
    }

    Some(CommandSuggestion {
        command: normalized.to_string(),
        confidence: 0.6,
        rationale: "Suggested as a copy-only diagnostic or repair step.".into(),
    })
}

fn command_heading_index(explanation: &str) -> Option<usize> {
    explanation.lines().position(|line| {
        let normalized = line
            .trim()
            .trim_start_matches('#')
            .trim()
            .to_ascii_lowercase();
        normalized.starts_with("optional command") || normalized.starts_with("suggested command")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ai_capability::contract::ChatProvider;
    use crate::core::ai_capability::test_fixtures::{
        assert_invalid_payload, assert_safe_primary_output, RUST_COMPILER_ERROR,
        TYPESCRIPT_STACK_TRACE,
    };
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    struct EchoProvider;
    impl ChatProvider for EchoProvider {
        fn chat(&self, _prompt: &str, _cancel: &AtomicBool) -> Result<String, String> {
            Ok(
                "Summary\nThe type mismatch blocks compilation.\n\nLikely cause\nThe value has the wrong type.\n\nCheck\nInspect the reported line.\n\nNext step\nRun a focused check.\n\nOptional command\n`cargo check`"
                    .into(),
            )
        }
    }
    struct EmptyProvider;
    impl ChatProvider for EmptyProvider {
        fn chat(&self, _prompt: &str, _cancel: &AtomicBool) -> Result<String, String> {
            Ok(String::new())
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
            id: CapabilityId::FixError,
            payload,
            context_hash: None,
        }
    }

    #[test]
    fn raw_output_path_parses_cargo_error() {
        let resp = call(
            req(serde_json::json!({
                "raw_output": RUST_COMPILER_ERROR,
            })),
            &deps(Arc::new(EchoProvider)),
        )
        .unwrap();
        assert!(!resp.risk_tag.requires_confirmation);
        assert_eq!(resp.id, CapabilityId::FixError);
        assert_safe_primary_output(&resp);
        match resp.output {
            CapabilityOutput::Structured { value } => {
                let parsed: FixErrorOutput = serde_json::from_value(value).unwrap();
                assert_eq!(parsed.suggested_command.unwrap().command, "cargo check");
            }
            _ => panic!("expected structured output"),
        }
    }

    #[test]
    fn apply_variant_is_unsupported() {
        let err = call(
            req(serde_json::json!({ "apply": { "path": "/x" } })),
            &deps(Arc::new(EchoProvider)),
        )
        .unwrap_err();
        assert!(matches!(err, CapabilityError::UnsupportedAction(_)));
    }

    #[test]
    fn empty_raw_output_is_invalid() {
        let err = call(
            req(serde_json::json!({ "raw_output": "   " })),
            &deps(Arc::new(EchoProvider)),
        )
        .unwrap_err();
        assert_invalid_payload(err);
    }

    #[test]
    fn rejects_malformed_payload_as_typed_error() {
        let err = call(
            req(serde_json::json!({ "raw_output": 9 })),
            &deps(Arc::new(EchoProvider)),
        )
        .unwrap_err();
        assert_invalid_payload(err);
    }

    #[test]
    fn run_command_rejects_unknown_program() {
        let err = call(
            req(serde_json::json!({
                "program": "rm",
                "args": ["-rf", "/"],
                "cwd": "."
            })),
            &deps(Arc::new(EchoProvider)),
        )
        .unwrap_err();
        assert!(matches!(err, CapabilityError::UnsupportedAction(_)));
    }

    #[test]
    fn stack_trace_fixture_returns_actionable_structured_output() {
        let resp = call(
            req(serde_json::json!({ "raw_output": TYPESCRIPT_STACK_TRACE })),
            &deps(Arc::new(EchoProvider)),
        )
        .unwrap();
        assert_safe_primary_output(&resp);
    }

    #[test]
    fn npm_and_cargo_failures_return_actionable_structured_output() {
        for raw_output in [
            "npm ERR! Missing script: \"test\"",
            "error: test failed, to rerun pass `--bin keynova`",
        ] {
            let resp = call(
                req(serde_json::json!({ "raw_output": raw_output })),
                &deps(Arc::new(EchoProvider)),
            )
            .unwrap();
            assert_safe_primary_output(&resp);
            assert!(matches!(resp.output, CapabilityOutput::Structured { .. }));
        }
    }

    #[test]
    fn state_changing_suggested_command_gets_confirm_risk() {
        struct InstallProvider;
        impl ChatProvider for InstallProvider {
            fn chat(&self, _prompt: &str, _cancel: &AtomicBool) -> Result<String, String> {
                Ok("Summary\nA package is missing.\n\nOptional command\n`npm install`".into())
            }
        }

        let resp = call(
            req(serde_json::json!({ "raw_output": "Cannot find module 'x'" })),
            &deps(Arc::new(InstallProvider)),
        )
        .unwrap();
        assert!(resp.risk_tag.requires_confirmation);
    }

    #[test]
    fn empty_provider_reply_is_typed_error() {
        let err = call(
            req(serde_json::json!({ "raw_output": "error: oops" })),
            &deps(Arc::new(EmptyProvider)),
        )
        .unwrap_err();
        assert!(matches!(err, CapabilityError::ProviderError(_)));
    }

    #[test]
    fn optional_command_parser_ignores_none() {
        let output = parse_output("Summary\nNo command needed.\n\nOptional command: none".into());
        assert!(output.suggested_command.is_none());
    }
}
