//! Shared representative inputs for capability contract tests.

use crate::core::ai_capability::{CapabilityError, CapabilityOutput, CapabilityResponse};

pub(crate) const RUST_COMPILER_ERROR: &str =
    "error[E0308]: mismatched types\n --> src/main.rs:8:12\n expected `String`, found `&str`";

pub(crate) const TYPESCRIPT_STACK_TRACE: &str =
    "TypeError: value.map is not a function\n    at renderList (src/list.ts:14:20)";

pub(crate) const CONFIG_SNIPPET: &str =
    "[package]\nname = \"keynova\"\nversion = \"0.7.0\"\n\n[features]\ndefault = []";

pub(crate) fn long_text() -> String {
    "Keynova keeps AI inline and task-focused. ".repeat(120)
}

pub(crate) const COMMAND_REQUEST: &str = "show the current git branch status";

pub(crate) fn assert_safe_primary_output(response: &CapabilityResponse) {
    let primary = match &response.output {
        CapabilityOutput::Text { text } => text.clone(),
        CapabilityOutput::Structured { value } => value.to_string(),
    }
    .to_ascii_lowercase();

    for forbidden in [
        "expected value at line",
        "expected ident at line",
        "failed to parse model json",
        "serde_json",
    ] {
        assert!(
            !primary.contains(forbidden),
            "raw parser failure leaked into primary output: {primary}"
        );
    }
}

pub(crate) fn assert_invalid_payload(error: CapabilityError) {
    assert!(matches!(error, CapabilityError::InvalidPayload(_)));
}
