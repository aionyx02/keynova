//! Capability: explain a compiler / linter error so the user can fix it.
//!
//! v1 is explanation-only per ADR-0029 §4 (initial three capabilities are
//! explanation-only). Payload variants that ask for file edits or shell
//! execution return `UnsupportedAction`. `audit = true`,
//! `requires_confirmation = false` (no destructive action).

use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::time::Duration;

use serde::Deserialize;

use crate::core::ai_capability::contract::{
    CapabilityDeps, CapabilityError, CapabilityOutput, CapabilityRequest, CapabilityResponse,
};
use crate::core::ai_capability::prompt::{build_prompt, maybe_audit};
use crate::core::ai_capability::registry::{meta, CapabilityId};
use crate::core::dev_runner::{
    extract_compiler_errors, run_bounded_dev_cmd, DEV_CARGO_TIMEOUT_SECS, DEV_NPM_TIMEOUT_SECS,
};
use crate::models::unified_result::RiskTag;

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

const SYSTEM: &str = "You are a senior developer's debugging assistant integrated into Keynova. \
     Given compiler or linter output, explain the most likely root cause in plain prose, then \
     suggest a one-line fix. Be concrete about file paths and line numbers when present. \
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
        "Explain the following compiler/lint error and suggest a one-line fix.\n\n{}",
        errors_block
    );
    let prompt = build_prompt(SYSTEM, &[], &task);

    let reply = match &deps.stream_chunk {
        Some(on_chunk) => deps
            .chat
            .chat_stream(&prompt, on_chunk.as_ref(), &deps.cancel),
        None => deps.chat.chat(&prompt, &deps.cancel),
    };

    let audit = meta(CapabilityId::FixError).audit;
    match reply {
        Ok(text) => {
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
                output: CapabilityOutput::Text { text },
                risk_tag: RiskTag::none(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ai_capability::contract::ChatProvider;
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    struct EchoProvider;
    impl ChatProvider for EchoProvider {
        fn chat(&self, _prompt: &str, _cancel: &AtomicBool) -> Result<String, String> {
            Ok("the type mismatch happens because…".into())
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
            id: CapabilityId::FixError,
            payload,
            context_hash: None,
        }
    }

    #[test]
    fn raw_output_path_parses_cargo_error() {
        let resp = call(
            req(serde_json::json!({
                "raw_output": "error[E0308]: mismatched types\n  --> src/x.rs:1:1\n",
            })),
            &deps(Arc::new(EchoProvider)),
        )
        .unwrap();
        assert!(!resp.risk_tag.requires_confirmation);
        assert_eq!(resp.id, CapabilityId::FixError);
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
        assert!(matches!(err, CapabilityError::InvalidPayload(_)));
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
}
