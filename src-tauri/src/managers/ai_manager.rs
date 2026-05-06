use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use crate::core::AppEvent;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiMessage {
    pub role: String,
    pub content: String,
}

/// Runtime AI provider selected from config.
#[derive(Debug, Clone)]
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
                        h.retain(|m| m.content != prompt || m.role != "user");
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

    #[test]
    fn normalizes_base_urls() {
        assert_eq!(
            normalize_base_url("https://example.test/v1/"),
            "https://example.test/v1"
        );
        assert_eq!(normalize_base_url(""), "https://api.openai.com/v1");
    }
}
