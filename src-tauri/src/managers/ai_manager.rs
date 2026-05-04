use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use crate::core::AppEvent;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiMessage {
    pub role: String,
    pub content: String,
}

/// Claude API 回應格式（部分欄位）。
#[derive(Deserialize)]
struct ClaudeResponse {
    content: Vec<ClaudeContent>,
}

#[derive(Deserialize)]
struct ClaudeContent {
    text: String,
}

/// AI 對話管理器：管理多輪對話歷史，使用 Claude API，透過 EventBus 推送結果。
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
        api_key: String,
        model: String,
        max_tokens: u32,
        timeout_secs: u64,
    ) {
        let publish = Arc::clone(&self.publish_event);
        let history = Arc::clone(&self.history);

        // Append user message immediately
        if let Ok(mut h) = history.lock() {
            h.push(AiMessage { role: "user".into(), content: prompt.clone() });
            // Keep last 20 messages to avoid token overflow
            let len = h.len();
            if len > 20 {
                h.drain(0..len - 20);
            }
        }

        let messages_snapshot: Vec<AiMessage> = history.lock()
            .map(|h| h.clone())
            .unwrap_or_default();

        std::thread::spawn(move || {
            let result = do_chat(&api_key, &model, max_tokens, timeout_secs, &messages_snapshot);
            match result {
                Ok(reply) => {
                    // Append assistant reply to history
                    if let Ok(mut h) = history.lock() {
                        h.push(AiMessage { role: "assistant".into(), content: reply.clone() });
                    }
                    publish(AppEvent::new("ai.response", serde_json::json!({
                        "request_id": request_id,
                        "ok": true,
                        "reply": reply,
                    })));
                }
                Err(e) => {
                    // Remove the user message we optimistically added
                    if let Ok(mut h) = history.lock() {
                        h.retain(|m| m.content != prompt || m.role != "user");
                    }
                    publish(AppEvent::new("ai.response", serde_json::json!({
                        "request_id": request_id,
                        "ok": false,
                        "error": e,
                    })));
                }
            }
        });
    }
}

fn do_chat(
    api_key: &str,
    model: &str,
    max_tokens: u32,
    timeout_secs: u64,
    messages: &[AiMessage],
) -> Result<String, String> {
    if api_key.is_empty() {
        return Err("AI API key not configured. Use /setting ai.api_key <key> to set it.".into());
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()
        .map_err(|e| e.to_string())?;

    let messages_json: Vec<serde_json::Value> = messages
        .iter()
        .map(|m| serde_json::json!({ "role": m.role, "content": m.content }))
        .collect();

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