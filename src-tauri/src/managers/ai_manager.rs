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
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AiToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters_schema: Value,
}

/// A model-requested tool call after provider-specific response normalization.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AiToolCallRequest {
    pub id: String,
    pub name: String,
    pub arguments: Value,
}

/// Observation returned to the model after Keynova executes or denies a tool request.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AiToolObservation {
    pub tool_call_id: String,
    pub content: String,
}

/// One provider turn in the future ReAct loop: either final text or tool requests.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AiToolTurn {
    FinalText { content: String },
    ToolCalls { tool_calls: Vec<AiToolCallRequest> },
}

#[allow(dead_code)]
pub fn provider_supports_tool_calls(provider: &AiProvider) -> bool {
    matches!(
        provider,
        AiProvider::OpenAI { .. } | AiProvider::Ollama { .. }
    )
}

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
