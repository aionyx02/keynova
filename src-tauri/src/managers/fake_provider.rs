use std::collections::VecDeque;
use std::sync::Mutex;

use serde_json::json;

use crate::managers::ai_manager::{
    AiMessage, AiToolCallRequest, AiToolDefinition, AiToolTurn, ToolCallError, ToolCallProvider,
};

/// Scripted `ToolCallProvider` for integration tests.
///
/// Returns pre-set turns in order; panics if asked for more turns than scripted.
pub struct FakeToolCallProvider {
    turns: Mutex<VecDeque<AiToolTurn>>,
}

impl FakeToolCallProvider {
    pub fn new(turns: impl IntoIterator<Item = AiToolTurn>) -> Self {
        Self {
            turns: Mutex::new(turns.into_iter().collect()),
        }
    }

    pub fn single_final(text: impl Into<String>) -> Self {
        Self::new([AiToolTurn::FinalText {
            content: text.into(),
        }])
    }

    pub fn tool_then_final(
        tool_name: impl Into<String>,
        tool_id: impl Into<String>,
        args: serde_json::Value,
        final_text: impl Into<String>,
    ) -> Self {
        Self::new([
            AiToolTurn::ToolCalls {
                tool_calls: vec![AiToolCallRequest {
                    id: tool_id.into(),
                    name: tool_name.into(),
                    arguments: args,
                }],
            },
            AiToolTurn::FinalText {
                content: final_text.into(),
            },
        ])
    }
}

impl ToolCallProvider for FakeToolCallProvider {
    fn chat_with_tools(
        &self,
        _messages: &[AiMessage],
        _tools: &[AiToolDefinition],
        _max_tokens: u32,
        _timeout_secs: u64,
    ) -> Result<AiToolTurn, ToolCallError> {
        self.turns
            .lock()
            .expect("FakeToolCallProvider mutex poisoned")
            .pop_front()
            .ok_or_else(|| ToolCallError::Network("FakeProvider: no more scripted turns".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_final_returns_text_then_errors() {
        let p = FakeToolCallProvider::single_final("done");
        let turn = p.chat_with_tools(&[], &[], 100, 5).unwrap();
        assert_eq!(turn, AiToolTurn::FinalText { content: "done".into() });
        assert!(p.chat_with_tools(&[], &[], 100, 5).is_err());
    }

    #[test]
    fn tool_then_final_sequences_correctly() {
        let p = FakeToolCallProvider::tool_then_final(
            "keynova_search",
            "call-1",
            json!({ "query": "notes" }),
            "Here are your notes.",
        );
        let first = p.chat_with_tools(&[], &[], 100, 5).unwrap();
        let second = p.chat_with_tools(&[], &[], 100, 5).unwrap();
        assert!(matches!(first, AiToolTurn::ToolCalls { .. }));
        assert_eq!(
            second,
            AiToolTurn::FinalText {
                content: "Here are your notes.".into()
            }
        );
    }
}