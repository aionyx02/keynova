use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::core::AppEvent;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiMessage {
    pub role: String,
    pub content: String,
}

/// Runtime AI provider selected from config.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AiProvider {
    Claude {
        api_key: String,
        model: String,
    },
    Ollama {
        base_url: String,
        model: String,
    },
    OpenAI {
        api_key: String,
        base_url: String,
        model: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiRuntimeConfig {
    pub provider: AiProvider,
    pub max_tokens: u32,
    pub timeout_secs: u64,
}

pub fn resolve_ai_runtime_config<F>(mut get: F) -> Result<AiRuntimeConfig, String>
where
    F: FnMut(&str) -> Option<String>,
{
    let provider_name = get("ai.provider")
        .unwrap_or_else(|| "claude".into())
        .trim()
        .to_lowercase();
    let model = selected_model_for_provider(&mut get, &provider_name);
    let max_tokens = get("ai.max_tokens")
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(4096);

    let provider = match provider_name.as_str() {
        "ollama" => AiProvider::Ollama {
            base_url: get("ai.ollama_url").unwrap_or_else(|| "http://localhost:11434".into()),
            model,
        },
        "openai" => AiProvider::OpenAI {
            api_key: get("ai.openai_api_key").unwrap_or_default(),
            base_url: get("ai.openai_base_url")
                .unwrap_or_else(|| "https://api.openai.com/v1".into()),
            model,
        },
        "claude" => AiProvider::Claude {
            api_key: get("ai.api_key").unwrap_or_default(),
            model,
        },
        other => return Err(format!("unsupported ai.provider '{other}'")),
    };

    let timeout = match &provider {
        AiProvider::Ollama { .. } => get("ai.ollama_timeout_secs")
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(120),
        _ => get("ai.timeout_secs")
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(30),
    };

    Ok(AiRuntimeConfig {
        provider,
        max_tokens,
        timeout_secs: timeout,
    })
}

fn selected_model_for_provider<F>(get: &mut F, provider_name: &str) -> String
where
    F: FnMut(&str) -> Option<String>,
{
    let active_model = get("ai.model").filter(|value| !value.trim().is_empty());
    match provider_name {
        "openai" => active_model
            .or_else(|| get("ai.openai_model"))
            .unwrap_or_else(|| "gpt-4o-mini".into()),
        "ollama" => active_model.unwrap_or_else(|| "qwen2.5:7b".into()),
        _ => active_model
            .or_else(|| get("ai.claude_model"))
            .unwrap_or_else(|| "claude-sonnet-4-6".into()),
    }
}

/// Provider-neutral tool definition offered to models that support function/tool calling.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AiToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters_schema: Value,
}

/// A model-requested tool call after provider-specific response normalization.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AiToolCallRequest {
    pub id: String,
    pub name: String,
    pub arguments: Value,
}

/// Observation returned to the model after Keynova executes or denies a tool request.
/// Used by provider implementations (A3/A4) to format tool results for their specific API.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AiToolObservation {
    pub tool_call_id: String,
    pub content: String,
}

/// One provider turn in the ReAct loop: either final text or tool requests.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AiToolTurn {
    FinalText { content: String },
    ToolCalls { tool_calls: Vec<AiToolCallRequest> },
}

/// Returns whether this provider variant is expected to support tool/function calling.
/// Used by `agent.start` (5.5.C) to select between heuristic and ReAct paths.
#[allow(dead_code)]
pub fn provider_supports_tool_calls(provider: &AiProvider) -> bool {
    matches!(
        provider,
        AiProvider::OpenAI { .. } | AiProvider::Ollama { .. }
    )
}

/// Error variants a `ToolCallProvider` can return.
#[derive(Debug, Clone)]
pub enum ToolCallError {
    /// This provider or model does not support function/tool calling.
    Unsupported,
    /// Network or API-level failure.
    Network(String),
    /// Could not parse the provider's response into a known format.
    Parse(String),
}

impl std::fmt::Display for ToolCallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported => write!(f, "provider does not support tool calls"),
            Self::Network(e) => write!(f, "network error: {e}"),
            Self::Parse(e) => write!(f, "parse error: {e}"),
        }
    }
}

/// Capability: respond to a message thread with tool schemas, returning either
/// a batch of model-requested tool calls or a final text answer.
///
/// Implementations: `FakeToolCallProvider` (tests), OpenAI-compatible (5.5.A3),
/// Ollama (5.5.A4).
pub trait ToolCallProvider: Send + Sync {
    fn chat_with_tools(
        &self,
        messages: &[AiMessage],
        tools: &[AiToolDefinition],
        max_tokens: u32,
        timeout_secs: u64,
    ) -> Result<AiToolTurn, ToolCallError>;
}

// ─── ToolCallProvider impl for AiProvider ───────────────────────────────────

impl ToolCallProvider for AiProvider {
    fn chat_with_tools(
        &self,
        messages: &[AiMessage],
        tools: &[AiToolDefinition],
        max_tokens: u32,
        timeout_secs: u64,
    ) -> Result<AiToolTurn, ToolCallError> {
        match self {
            AiProvider::OpenAI {
                api_key,
                base_url,
                model,
            } => openai_chat_with_tools(api_key, base_url, model, messages, tools, max_tokens, timeout_secs),
            AiProvider::Ollama { base_url, model } => {
                ollama_chat_with_tools(base_url, model, messages, tools, max_tokens, timeout_secs)
            }
            AiProvider::Claude { .. } => Err(ToolCallError::Unsupported),
        }
    }
}

// ─── Response structs for tool-call APIs ────────────────────────────────────

/// Claude API response format used by the messages endpoint.
#[derive(Deserialize)]
struct ClaudeResponse {
    content: Vec<ClaudeContent>,
}

#[derive(Deserialize)]
struct ClaudeContent {
    text: String,
}

#[derive(Deserialize)]
struct OllamaChatResponse {
    message: OllamaMessage,
}

#[derive(Deserialize)]
struct OllamaMessage {
    content: String,
}

#[derive(Deserialize)]
struct OpenAiResponse {
    choices: Vec<OpenAiChoice>,
}

#[derive(Deserialize)]
struct OpenAiChoice {
    message: OpenAiMessage,
}

#[derive(Deserialize)]
struct OpenAiMessage {
    content: Option<String>,
}

// ─── Tool-call response structs ──────────────────────────────────────────────

/// OpenAI-format tool-call response (shared by OpenAI and OpenAI-compatible endpoints).
#[derive(Deserialize)]
struct OpenAiToolsResponse {
    choices: Vec<OpenAiToolsChoice>,
}

#[derive(Deserialize)]
struct OpenAiToolsChoice {
    message: OpenAiToolsMessage,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct OpenAiToolsMessage {
    content: Option<String>,
    tool_calls: Option<Vec<OpenAiToolCallEntry>>,
}

#[derive(Deserialize)]
struct OpenAiToolCallEntry {
    id: String,
    function: OpenAiToolCallFunction,
}

#[derive(Deserialize)]
struct OpenAiToolCallFunction {
    name: String,
    /// Arguments are a JSON-encoded string in the OpenAI format.
    arguments: String,
}

/// Ollama tool-call response (Ollama ≥ 0.3 / OpenAI-compatible endpoint).
#[derive(Deserialize)]
struct OllamaToolsResponse {
    message: OllamaToolsMessage,
}

#[derive(Deserialize)]
struct OllamaToolsMessage {
    content: Option<String>,
    tool_calls: Option<Vec<OllamaToolCallEntry>>,
}

#[derive(Deserialize)]
struct OllamaToolCallEntry {
    function: OllamaToolCallFunction,
}

#[derive(Deserialize)]
struct OllamaToolCallFunction {
    name: String,
    /// Ollama returns arguments as a pre-parsed JSON object (not a string).
    arguments: Value,
}

/// AI 對話管理器：管理多輪對話歷史，依 config 選擇 Claude/Ollama/OpenAI provider。
pub struct AiManager {
    publish_event: Arc<dyn Fn(AppEvent) + Send + Sync>,
    /// 多輪對話歷史（全域共享，前端可呼叫 clear_history 重置）
    history: Arc<Mutex<Vec<AiMessage>>>,
}

impl AiManager {
    pub fn new(publish_event: Arc<dyn Fn(AppEvent) + Send + Sync>) -> Self {
        Self {
            publish_event,
            history: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// 取得對話歷史。
    pub fn get_history(&self) -> Vec<AiMessage> {
        self.history.lock().map(|h| h.clone()).unwrap_or_default()
    }

    /// 清除對話歷史。
    pub fn clear_history(&self) {
        if let Ok(mut h) = self.history.lock() {
            h.clear();
        }
    }

    /// 非同步對話：立即回傳 request_id，結果透過 ai.response 事件推送。
    pub fn chat_async(
        &self,
        request_id: String,
        prompt: String,
        provider: AiProvider,
        max_tokens: u32,
        timeout_secs: u64,
    ) {
        let publish = Arc::clone(&self.publish_event);
        let history = Arc::clone(&self.history);

        if let Ok(mut h) = history.lock() {
            h.push(AiMessage {
                role: "user".into(),
                content: prompt.clone(),
            });
            let len = h.len();
            if len > 20 {
                h.drain(0..len - 20);
            }
        }

        let messages_snapshot: Vec<AiMessage> =
            history.lock().map(|h| h.clone()).unwrap_or_default();

        std::thread::spawn(move || {
            let result = do_chat(&provider, max_tokens, timeout_secs, &messages_snapshot);
            match result {
                Ok(reply) => {
                    if let Ok(mut h) = history.lock() {
                        h.push(AiMessage {
                            role: "assistant".into(),
                            content: reply.clone(),
                        });
                    }
                    publish(AppEvent::new(
                        "ai.response",
                        serde_json::json!({
                            "request_id": request_id,
                            "ok": true,
                            "reply": reply,
                        }),
                    ));
                }
                Err(e) => {
                    if let Ok(mut h) = history.lock() {
                        if let Some(pos) = h.iter().rposition(|m| m.role == "user" && m.content == prompt) {
                            h.remove(pos);
                        }
                    }
                    publish(AppEvent::new(
                        "ai.response",
                        serde_json::json!({
                            "request_id": request_id,
                            "ok": false,
                            "error": e,
                        }),
                    ));
                }
            }
        });
    }
}

fn do_chat(
    provider: &AiProvider,
    max_tokens: u32,
    timeout_secs: u64,
    messages: &[AiMessage],
) -> Result<String, String> {
    match provider {
        AiProvider::Claude { api_key, model } => {
            chat_claude(api_key, model, max_tokens, timeout_secs, messages)
        }
        AiProvider::Ollama { base_url, model } => {
            chat_ollama(base_url, model, max_tokens, timeout_secs, messages)
        }
        AiProvider::OpenAI {
            api_key,
            base_url,
            model,
        } => chat_openai(api_key, base_url, model, max_tokens, timeout_secs, messages),
    }
}

fn chat_claude(
    api_key: &str,
    model: &str,
    max_tokens: u32,
    timeout_secs: u64,
    messages: &[AiMessage],
) -> Result<String, String> {
    if api_key.trim().is_empty() {
        return Err("AI API key not configured. Use /setting ai.api_key <key> to set it.".into());
    }

    let client = build_client(timeout_secs)?;
    let messages_json = messages_json(messages);
    let body = serde_json::json!({
        "model": model,
        "max_tokens": max_tokens,
        "system": "You are a helpful assistant integrated into Keynova, a keyboard-centric productivity launcher. Be concise and practical.",
        "messages": messages_json,
    });

    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .map_err(|e| format!("request failed: {e}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body_text = response.text().unwrap_or_default();
        return Err(format!("API error {status}: {body_text}"));
    }

    let resp: ClaudeResponse = response.json().map_err(|e| e.to_string())?;
    resp.content
        .into_iter()
        .next()
        .map(|c| c.text.trim().to_string())
        .ok_or_else(|| "empty response from API".into())
}

fn chat_ollama(
    base_url: &str,
    model: &str,
    max_tokens: u32,
    timeout_secs: u64,
    messages: &[AiMessage],
) -> Result<String, String> {
    let client = build_client(timeout_secs)?;
    let url = format!("{}/api/chat", normalize_base_url(base_url));
    let body = serde_json::json!({
        "model": model,
        "stream": false,
        "messages": messages_json(messages),
        "options": {
            "num_predict": max_tokens,
        },
    });

    let response = client
        .post(url)
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .map_err(|e| format!("Ollama request failed: {e}. For large local models, increase ai.ollama_timeout_secs."))?;

    if !response.status().is_success() {
        let status = response.status();
        let body_text = response.text().unwrap_or_default();
        return Err(format!("Ollama error {status}: {body_text}"));
    }

    let resp: OllamaChatResponse = response.json().map_err(|e| e.to_string())?;
    let reply = resp.message.content.trim().to_string();
    if reply.is_empty() {
        Err("empty response from Ollama".into())
    } else {
        Ok(reply)
    }
}

fn chat_openai(
    api_key: &str,
    base_url: &str,
    model: &str,
    max_tokens: u32,
    timeout_secs: u64,
    messages: &[AiMessage],
) -> Result<String, String> {
    if api_key.trim().is_empty() {
        return Err("OpenAI-compatible API key not configured.".into());
    }

    let client = build_client(timeout_secs)?;
    let url = format!("{}/chat/completions", normalize_base_url(base_url));
    let body = serde_json::json!({
        "model": model,
        "max_tokens": max_tokens,
        "messages": messages_json(messages),
    });

    let response = client
        .post(url)
        .bearer_auth(api_key)
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .map_err(|e| format!("request failed: {e}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body_text = response.text().unwrap_or_default();
        return Err(format!("OpenAI-compatible API error {status}: {body_text}"));
    }

    let resp: OpenAiResponse = response.json().map_err(|e| e.to_string())?;
    resp.choices
        .into_iter()
        .find_map(|choice| choice.message.content)
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
        .ok_or_else(|| "empty response from OpenAI-compatible API".into())
}

// ─── chat_with_tools implementations ────────────────────────────────────────

fn openai_chat_with_tools(
    api_key: &str,
    base_url: &str,
    model: &str,
    messages: &[AiMessage],
    tools: &[AiToolDefinition],
    max_tokens: u32,
    timeout_secs: u64,
) -> Result<AiToolTurn, ToolCallError> {
    if api_key.trim().is_empty() {
        return Err(ToolCallError::Network(
            "OpenAI-compatible API key is not configured.".into(),
        ));
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()
        .map_err(|e| ToolCallError::Network(e.to_string()))?;

    let url = format!("{}/chat/completions", normalize_base_url(base_url));
    let tools_json: Vec<Value> = tools.iter().map(tool_def_to_openai).collect();

    let body = serde_json::json!({
        "model": model,
        "max_tokens": max_tokens,
        "messages": messages_json(messages),
        "tools": tools_json,
        "tool_choice": "auto",
    });

    let response = client
        .post(&url)
        .bearer_auth(api_key)
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .map_err(|e| ToolCallError::Network(format!("request failed: {e}")))?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().unwrap_or_default();
        return Err(ToolCallError::Network(format!(
            "OpenAI-compatible API error {status}: {text}"
        )));
    }

    let resp: OpenAiToolsResponse = response
        .json()
        .map_err(|e| ToolCallError::Parse(format!("response parse failed: {e}")))?;

    let choice = resp
        .choices
        .into_iter()
        .next()
        .ok_or_else(|| ToolCallError::Parse("empty choices array".into()))?;

    // Tool-call response takes priority over content.
    if let Some(tool_calls) = choice.message.tool_calls.filter(|v| !v.is_empty()) {
        let parsed = parse_openai_tool_calls(tool_calls)?;
        return Ok(AiToolTurn::ToolCalls { tool_calls: parsed });
    }

    // Plain text answer.
    let content = choice
        .message
        .content
        .unwrap_or_default()
        .trim()
        .to_string();

    if content.is_empty() && choice.finish_reason.as_deref() == Some("length") {
        return Err(ToolCallError::Parse(
            "response truncated (finish_reason=length)".into(),
        ));
    }

    Ok(AiToolTurn::FinalText { content })
}

fn ollama_chat_with_tools(
    base_url: &str,
    model: &str,
    messages: &[AiMessage],
    tools: &[AiToolDefinition],
    max_tokens: u32,
    timeout_secs: u64,
) -> Result<AiToolTurn, ToolCallError> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()
        .map_err(|e| ToolCallError::Network(e.to_string()))?;

    let url = format!("{}/api/chat", normalize_base_url(base_url));
    let tools_json: Vec<Value> = tools.iter().map(tool_def_to_openai).collect();

    let body = serde_json::json!({
        "model": model,
        "stream": false,
        "messages": messages_json(messages),
        "tools": tools_json,
        "options": { "num_predict": max_tokens },
    });

    let response = client
        .post(&url)
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .map_err(|e| {
            ToolCallError::Network(format!(
                "Ollama request failed: {e}. Is the Ollama daemon running?"
            ))
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().unwrap_or_default();
        // Older Ollama versions return 400 when a model doesn't support tools.
        if status.as_u16() == 400 && text.contains("does not support tools") {
            return Err(ToolCallError::Unsupported);
        }
        return Err(ToolCallError::Network(format!(
            "Ollama error {status}: {text}"
        )));
    }

    let resp: OllamaToolsResponse = response
        .json()
        .map_err(|e| ToolCallError::Parse(format!("response parse failed: {e}")))?;

    // Tool calls take priority over content.
    if let Some(tool_calls) = resp.message.tool_calls.filter(|v| !v.is_empty()) {
        let parsed = parse_ollama_tool_calls(tool_calls)?;
        return Ok(AiToolTurn::ToolCalls { tool_calls: parsed });
    }

    let content = resp
        .message
        .content
        .unwrap_or_default()
        .trim()
        .to_string();

    // If content is also empty the model likely ignored the tool schema.
    if content.is_empty() {
        return Err(ToolCallError::Unsupported);
    }

    Ok(AiToolTurn::FinalText { content })
}

/// Convert an `AiToolDefinition` to the OpenAI function-calling schema format.
fn tool_def_to_openai(def: &AiToolDefinition) -> Value {
    serde_json::json!({
        "type": "function",
        "function": {
            "name": def.name,
            "description": def.description,
            "parameters": def.parameters_schema,
        }
    })
}

/// Parse OpenAI-format tool calls where `arguments` is a JSON-encoded string.
fn parse_openai_tool_calls(
    calls: Vec<OpenAiToolCallEntry>,
) -> Result<Vec<AiToolCallRequest>, ToolCallError> {
    calls
        .into_iter()
        .map(|call| {
            let arguments = serde_json::from_str::<Value>(&call.function.arguments)
                .map_err(|e| {
                    ToolCallError::Parse(format!(
                        "invalid arguments JSON for '{}': {e}",
                        call.function.name
                    ))
                })?;
            Ok(AiToolCallRequest {
                id: call.id,
                name: call.function.name,
                arguments,
            })
        })
        .collect()
}

/// Parse Ollama-format tool calls where `arguments` is already a JSON object.
fn parse_ollama_tool_calls(
    calls: Vec<OllamaToolCallEntry>,
) -> Result<Vec<AiToolCallRequest>, ToolCallError> {
    calls
        .into_iter()
        .enumerate()
        .map(|(idx, call)| {
            Ok(AiToolCallRequest {
                id: format!("ollama-call-{idx}"),
                name: call.function.name,
                arguments: call.function.arguments,
            })
        })
        .collect()
}

fn build_client(timeout_secs: u64) -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()
        .map_err(|e| e.to_string())
}

fn messages_json(messages: &[AiMessage]) -> Vec<serde_json::Value> {
    messages
        .iter()
        .map(|m| serde_json::json!({ "role": m.role, "content": m.content }))
        .collect()
}

fn normalize_base_url(base_url: &str) -> String {
    let trimmed = base_url.trim();
    if trimmed.is_empty() {
        "https://api.openai.com/v1".to_string()
    } else {
        trimmed.trim_end_matches('/').to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn normalizes_base_urls() {
        assert_eq!(
            normalize_base_url("https://example.test/v1/"),
            "https://example.test/v1"
        );
        assert_eq!(normalize_base_url(""), "https://api.openai.com/v1");
    }

    #[test]
    fn selected_openai_or_ollama_provider_can_drive_agent_tool_calls() {
        assert!(provider_supports_tool_calls(&AiProvider::OpenAI {
            api_key: "key".into(),
            base_url: "https://api.openai.com/v1".into(),
            model: "gpt-4o-mini".into(),
        }));
        assert!(provider_supports_tool_calls(&AiProvider::Ollama {
            base_url: "http://localhost:11434".into(),
            model: "qwen2.5:7b".into(),
        }));
        assert!(!provider_supports_tool_calls(&AiProvider::Claude {
            api_key: "key".into(),
            model: "claude-sonnet-4-6".into(),
        }));
    }

    #[test]
    fn resolves_selected_ollama_runtime_config_without_api_key() {
        let config = HashMap::from([
            ("ai.provider", "ollama"),
            ("ai.model", "qwen2.5:7b"),
            ("ai.ollama_url", "http://localhost:11434"),
            ("ai.ollama_timeout_secs", "180"),
            ("ai.max_tokens", "2048"),
        ]);

        let resolved =
            resolve_ai_runtime_config(|key| config.get(key).map(|value| value.to_string()))
                .expect("resolve config");

        assert_eq!(
            resolved,
            AiRuntimeConfig {
                provider: AiProvider::Ollama {
                    base_url: "http://localhost:11434".into(),
                    model: "qwen2.5:7b".into(),
                },
                max_tokens: 2048,
                timeout_secs: 180,
            }
        );
    }

    // ─── tool-call parsing tests (5.5.A3 / A4) ───────────────────────────────

    #[test]
    fn tool_def_serialized_to_openai_function_schema() {
        let def = AiToolDefinition {
            name: "keynova_search".into(),
            description: "Search Keynova.".into(),
            parameters_schema: serde_json::json!({
                "type": "object",
                "properties": { "query": { "type": "string" } },
                "required": ["query"],
            }),
        };
        let schema = tool_def_to_openai(&def);
        assert_eq!(schema["type"], "function");
        assert_eq!(schema["function"]["name"], "keynova_search");
        assert_eq!(schema["function"]["parameters"]["properties"]["query"]["type"], "string");
    }

    #[test]
    fn parse_openai_tool_calls_decodes_arguments_string() {
        let raw = vec![OpenAiToolCallEntry {
            id: "call-1".into(),
            function: OpenAiToolCallFunction {
                name: "keynova_search".into(),
                arguments: r#"{"query":"rust notes","limit":5}"#.into(),
            },
        }];
        let parsed = parse_openai_tool_calls(raw).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].id, "call-1");
        assert_eq!(parsed[0].name, "keynova_search");
        assert_eq!(parsed[0].arguments["query"], "rust notes");
        assert_eq!(parsed[0].arguments["limit"], 5);
    }

    #[test]
    fn parse_openai_tool_calls_returns_parse_error_on_invalid_json() {
        let raw = vec![OpenAiToolCallEntry {
            id: "call-2".into(),
            function: OpenAiToolCallFunction {
                name: "keynova_search".into(),
                arguments: "NOT_JSON".into(),
            },
        }];
        let result = parse_openai_tool_calls(raw);
        assert!(matches!(result, Err(ToolCallError::Parse(_))));
    }

    #[test]
    fn parse_ollama_tool_calls_uses_pre_parsed_json() {
        let raw = vec![OllamaToolCallEntry {
            function: OllamaToolCallFunction {
                name: "keynova_search".into(),
                arguments: serde_json::json!({"query": "notes", "limit": 3}),
            },
        }];
        let parsed = parse_ollama_tool_calls(raw).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].id, "ollama-call-0");
        assert_eq!(parsed[0].arguments["limit"], 3);
    }

    #[test]
    fn claude_provider_returns_unsupported() {
        let provider = AiProvider::Claude {
            api_key: "key".into(),
            model: "claude-sonnet-4-6".into(),
        };
        let result = provider.chat_with_tools(&[], &[], 100, 5);
        assert!(matches!(result, Err(ToolCallError::Unsupported)));
    }

    #[test]
    fn resolves_openai_model_from_provider_specific_fallback() {
        let config = HashMap::from([
            ("ai.provider", "openai"),
            ("ai.openai_model", "gpt-4o-mini"),
            ("ai.openai_api_key", "key"),
            ("ai.openai_base_url", "https://api.openai.com/v1"),
        ]);

        let resolved =
            resolve_ai_runtime_config(|key| config.get(key).map(|value| value.to_string()))
                .expect("resolve config");

        assert_eq!(
            resolved.provider,
            AiProvider::OpenAI {
                api_key: "key".into(),
                base_url: "https://api.openai.com/v1".into(),
                model: "gpt-4o-mini".into(),
            }
        );
    }
}
